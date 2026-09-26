use tracing::info;

use crate::config::Environment;
use crate::exchanges::{
    registry::{default_dev_accounts, ExchangeRegistry},
    resources::ResourceManager,
    rest::RestUse,
    router::{default_live_streams, rest_use_for_need, MarketNeed},
    stream::{StreamEvent, StreamKind},
    ws::{WsConfig, WsSessionPlan},
    ExchangeAccountId, ExchangeError,
};

/// Logs the transport and resource matrix for the active spot account at startup.
pub fn log_transport_plan(
    registry: &ExchangeRegistry,
    environment: Environment,
    spot: &crate::exchanges::registry::AccountRegistration,
    symbol: &str,
) {
    let _ = registry.get(&spot.id);
    let dev_template_accounts = default_dev_accounts().len();

    info!(
        target: "exchanges",
        supported_exchanges = ?ResourceManager::supported_exchanges(),
        supported_markets = ?ResourceManager::supported_markets(),
        dev_template_accounts,
        "Exchange resource catalog"
    );

    let mut manager = ResourceManager::default();
    for resource in ResourceManager::defaults_for_account(&spot.id) {
        manager.upsert(resource);
    }
    let armed = manager.for_account(&spot.id);
    info!(
        target: "exchanges",
        armed_resources = armed.len(),
        "Managed resources registered for spot account"
    );

    for need in [
        MarketNeed::HistoricalBackfill,
        MarketNeed::LiveIncremental,
        MarketNeed::OrderLifecycle,
        MarketNeed::BalanceSnapshot,
        MarketNeed::UserData,
    ] {
        let transport = crate::exchanges::router::route_market_need(&spot.id, need);
        let rest = rest_use_for_need(need);
        info!(
            target: "exchanges",
            need = ?need,
            transport = ?transport,
            rest_use = ?rest,
            "Market need routing"
        );
    }

    for rest_use in [
        RestUse::HistoricalBackfill,
        RestUse::BalanceSnapshot,
        RestUse::OrderSubmit,
        RestUse::OrderStatus,
        RestUse::OrderCancel,
    ] {
        let _ = rest_use.transport();
        let _ = rest_use.requires_execution_gate();
    }

    let ws_config = WsConfig::default();
    if let Err(error) = ws_config.validate() {
        info!(target: "exchanges", %error, "Default websocket config invalid");
    } else {
        let _delay = ws_config.reconnect_delay(1);
    }

    let streams = default_live_streams(&spot.id, symbol);
    info!(
        target: "exchanges",
        live_stream_subscriptions = streams.len(),
        "Default live stream plan (not armed in REST polling mode)"
    );
    if let Ok(plan) = build_ws_session_plan(&spot.id, symbol, &streams) {
        if let Err(error) = plan.validate() {
            info!(target: "exchanges", %error, "Websocket session plan validation failed");
        }
    }

    if let Ok(event) = sample_stream_event(&spot.id, symbol) {
        if let Err(error) = event.validate() {
            info!(target: "exchanges", %error, "Sample stream event rejected");
        }
    }

    let _futures_registered = !registry.futures_accounts(environment).is_empty();
}

fn build_ws_session_plan(
    _account: &ExchangeAccountId,
    symbol: &str,
    subscriptions: &[crate::exchanges::stream::StreamSubscription],
) -> Result<WsSessionPlan, ExchangeError> {
    if subscriptions.is_empty() {
        return Err(ExchangeError::StreamDisconnected);
    }
    Ok(WsSessionPlan {
        endpoint: format!("wss://stream.binance.com/ws/{symbol}@kline_1m"),
        config: WsConfig::default(),
        subscriptions: subscriptions.to_vec(),
    })
}

fn sample_stream_event(
    account: &ExchangeAccountId,
    symbol: &str,
) -> Result<StreamEvent, ExchangeError> {
    if symbol.trim().is_empty() {
        return Err(ExchangeError::StreamDisconnected);
    }
    Ok(StreamEvent {
        account: account.clone(),
        stream: StreamKind::MiniTicker,
        symbol: symbol.to_owned(),
        event_ms: 0,
        payload: serde_json::json!({}),
    })
}
