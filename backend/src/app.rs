use std::sync::Arc;
use tokio::{
    sync::mpsc,
    time::{self, Duration},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::ui::{self, AppEvent, Dashboard, UiCommand};
use crate::{
    config::Config,
    domain::BotSignal,
    error::{BotError, BotResult},
    exchanges::{
        binance::BinanceMarketData,
        bootstrap::{load_registry, registered_market_types, spot_account_for_symbol},
        capabilities::catalog,
        live::{market_stream_plan, spawn_market_kline_stream, ClosedKline},
        preflight::log_transport_plan,
        resources::ResourceManager,
        rest::{authorize_rest_use, RestUse},
        router::{rest_use_for_need, route_market_need, MarketNeed},
        ExchangeAccountId, MarketDataSource,
    },
    jev::JevAdvisor,
    market::{Candle, HistoricalDataset},
    market_feed::{HybridCandleFeed, ws_matches_configured_timeframe},
    persistence::{persist_market_data_enabled, Database},
    portfolio::Asset,
    risk::{gate_signal, profile_limits, ExecutionContext, RiskLimits},
    strategy::{evaluate, Signal, StrategySnapshot},
};

async fn persist_polled_1m_window(
    db: &Database,
    symbol: &str,
    candles: Vec<mantis_ta::types::Candle>,
) {
    match HistoricalDataset::from_mantis_1m(symbol, "monitor-rest", candles) {
        Ok(dataset) => {
            if let Err(error) = db.persist_dataset(&dataset).await {
                warn!(target: "persistence", error=%error, "Failed to persist polled 1m candles");
            }
        }
        Err(error) => {
            warn!(target: "persistence", error=%error, "Skipped persist: polled window is not a valid 1m series");
        }
    }
}

async fn persist_ws_1m_candle(db: &Database, symbol: &str, candle: Candle) {
    match HistoricalDataset::from_1m(symbol, "monitor-ws", vec![candle]) {
        Ok(dataset) => {
            if let Err(error) = db.persist_dataset(&dataset).await {
                warn!(target: "persistence", error=%error, "Failed to persist WS closed 1m candle");
            }
        }
        Err(error) => {
            warn!(target: "persistence", error=%error, "Skipped persist: WS candle failed validation");
        }
    }
}

async fn jev_note(advisor: &Option<JevAdvisor>, snapshot: &StrategySnapshot) -> Option<String> {
    if let Some(jev) = advisor {
        match jev.review(snapshot).await {
            Ok(reviews) => Some(reviews.join(" | ")),
            Err(e) => {
                warn!(target: "jev", error=%e, "Jev unavailable; advisory omitted; strategy execution remains independently gated");
                None
            }
        }
    } else {
        None
    }
}

async fn apply_strategy_snapshot(
    snapshot: StrategySnapshot,
    note: Option<String>,
    feed_source: &str,
    config: &Config,
    limits: RiskLimits,
    spot_account_id: &ExchangeAccountId,
    execution_ctx: &mut ExecutionContext,
    dashboard: &mut Dashboard,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    info!(
        target: "market",
        symbol=%config.market.symbol,
        close=snapshot.close,
        candle_ts=snapshot.candle_timestamp_ms,
        feed_source,
        "Market candle evaluated"
    );
    let bot_signal = BotSignal {
        bot_id: None,
        signal: snapshot.signal,
    };
    info!(target: "strategy", bot_signal=?bot_signal, fast_sma=?snapshot.fast_sma, slow_sma=?snapshot.slow_sma, "Strategy state");
    if matches!(snapshot.signal, Signal::Buy | Signal::Sell) {
        let _live = route_market_need(spot_account_id, MarketNeed::LiveIncremental);
        let _orders = route_market_need(spot_account_id, MarketNeed::OrderLifecycle);
        if let Some(rest_use) = rest_use_for_need(MarketNeed::OrderLifecycle) {
            if rest_use.requires_execution_gate() {
                let _ = authorize_rest_use(spot_account_id, rest_use);
            }
        }
        if RestUse::OrderSubmit.requires_execution_gate() {
            if let Err(e) = authorize_rest_use(spot_account_id, RestUse::OrderSubmit) {
                info!(target: "exchanges", error=%e, transport=?RestUse::OrderSubmit.transport(), "Order REST path blocked by execution policy");
            }
        }
        match gate_signal(snapshot.signal, limits, config.run_mode, execution_ctx) {
            Ok(()) => warn!(
                target: "risk",
                environment=%config.environment,
                mode=%config.run_mode,
                quote_cap=limits.max_order_quote,
                open_positions=execution_ctx.open_positions,
                "Strategy intent passed risk gate; live orders remain disabled in v1"
            ),
            Err(e) => warn!(target: "risk", error=%e, "Strategy intent rejected by risk gate"),
        }
    }
    dashboard.set_paper_open_positions(execution_ctx.open_positions);
    dashboard.update_market(snapshot, note);
    dashboard.push_log(format!(
        "{} | {} | {} | {}",
        config.market.symbol,
        config.operation,
        config.risk_profile,
        feed_source
    ));
    event_tx.send(AppEvent::Refresh(dashboard.clone())).await.is_ok()
}

async fn run_evaluation_cycle(
    feed: &mut HybridCandleFeed,
    candle_ts: i64,
    feed_source: &str,
    config: &Config,
    limits: RiskLimits,
    advisor: &Option<JevAdvisor>,
    spot_account_id: &ExchangeAccountId,
    execution_ctx: &mut ExecutionContext,
    dashboard: &mut Dashboard,
    event_tx: &mpsc::Sender<AppEvent>,
) -> bool {
    let snapshot = evaluate(
        feed.candles(),
        config.strategy.sma_fast,
        config.strategy.sma_slow,
    );
    let note = jev_note(advisor, &snapshot).await;
    feed.mark_evaluated(candle_ts);
    apply_strategy_snapshot(
        snapshot,
        note,
        feed_source,
        config,
        limits,
        spot_account_id,
        execution_ctx,
        dashboard,
        event_tx,
    )
    .await
}

async fn recv_ws_kline(rx: &mut Option<mpsc::Receiver<ClosedKline>>) -> Option<ClosedKline> {
    match rx {
        Some(channel) => channel.recv().await,
        None => std::future::pending().await,
    }
}

pub async fn run(config: Config, database: Option<Database>) -> BotResult<()> {
    let registry = load_registry(config.environment)
        .map_err(|e| crate::error::BotError::Configuration(e.to_string()))?;
    let spot = spot_account_for_symbol(&registry, config.environment, &config.market.symbol)?;
    info!(
        target: "exchanges",
        account_key = %spot.id.key(),
        rate_limit = spot.rate_limit_per_minute,
        markets = ?registered_market_types(&registry, config.environment),
        "Exchange registry loaded"
    );
    log_transport_plan(&registry, config.environment, spot, &config.market.symbol);
    if let Err(error) = authorize_rest_use(&spot.id, RestUse::HistoricalBackfill) {
        warn!(target: "exchanges", error=%error, "Historical REST backfill gate rejected");
    }
    // Hybrid feed: WS closed 1m klines trigger evaluation; REST poll refreshes the window on interval.
    let ws_shutdown = CancellationToken::new();
    let ws_for_timeframe = ws_matches_configured_timeframe(&config.market.timeframe);
    let (mut closed_kline_rx, ws_stream_task) = if ws_for_timeframe {
        let (closed_kline_tx, closed_kline_rx) = mpsc::channel::<ClosedKline>(64);
        let task = match market_stream_plan(&spot.id, &config.market.symbol, config.environment)
        {
            Ok(plan) => Some(spawn_market_kline_stream(
                plan,
                ws_shutdown.clone(),
                Some(closed_kline_tx),
            )),
            Err(error) => {
                warn!(target: "exchanges::ws", error=%error, "WS market kline stream not started");
                None
            }
        };
        (Some(closed_kline_rx), task)
    } else {
        info!(
            target: "exchanges::ws",
            timeframe = %config.market.timeframe,
            "Websocket kline stream disabled; only REST poll drives strategy for this timeframe"
        );
        (None, None)
    };
    if persist_market_data_enabled() && database.is_none() {
        warn!(
            target: "persistence",
            "PERSIST_MARKET_DATA=1 but DATABASE_URL is unset or connection failed; ingest disabled"
        );
    }
    if let Some(binance) = catalog().get(&spot.id.exchange) {
        info!(target: "exchanges", notes = %binance.notes, "Exchange capability catalog");
    }
    if let Some(quote) = config.market.symbol.split('/').nth(1) {
        if let Ok(asset) = Asset::new(quote) {
            let template = crate::portfolio::paper_snapshot(&asset);
            let _ = template.validate();
            let _ = template.balance(&asset);
            info!(target: "portfolio", quote_asset = asset.as_str(), "Quote asset for configured symbol");
        }
    }
    for resource in ResourceManager::defaults_for_account(&spot.id) {
        if resource.enabled {
            info!(target: "exchanges", resource = %resource.name, kind = ?resource.kind, "Managed resource armed");
        }
    }

    let credentials = config.credentials()?;
    if credentials.api_key.is_some() {
        info!(target: "security", "Binance testnet credentials loaded from environment; values are redacted");
    } else {
        warn!(target: "security", "No testnet keys found; public market data only");
    }
    let exchange = Arc::new(BinanceMarketData::new(credentials, config.environment)?);
    let advisor = JevAdvisor::from_env(config.jev.clone())?;
    if advisor.is_some() {
        info!(target: "jev", "Jev advisor enabled for configured advisory questions");
    }
    let base_limits = RiskLimits {
        max_order_quote: config.risk.max_order_quote,
        max_daily_loss_quote: config.risk.max_daily_loss_quote,
        max_open_positions: config.risk.max_open_positions,
    };
    let limits = profile_limits(config.risk_profile, base_limits);
    let mut dashboard = Dashboard::new(config.clone(), limits);
    let (event_tx, event_rx) = mpsc::channel(64);
    let (command_tx, mut command_rx) = mpsc::channel(8);
    let ui_task = tokio::spawn(ui::run(dashboard.clone(), command_tx.clone(), event_rx));

    let spot_account_id = spot.id.clone();
    let mut execution_ctx = ExecutionContext::default();
    let mut interval = time::interval(Duration::from_secs(config.market.poll_seconds));
    let mut paused = false;
    let mut persist_timeframe_warned = false;
    let mut hybrid_feed = HybridCandleFeed::new(config.market.candle_limit);
    let ws_feed_active = closed_kline_rx.is_some();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            cmd = command_rx.recv() => match cmd {
                Some(UiCommand::Quit) | None => break,
                Some(UiCommand::Pause) => { paused = true; info!(target: "system", "Trading evaluation paused by operator"); }
                Some(UiCommand::Resume) => { paused = false; info!(target: "system", "Trading evaluation resumed by operator"); }
            },
            kline = recv_ws_kline(&mut closed_kline_rx), if !paused && ws_feed_active => {
                if persist_market_data_enabled() {
                    if let Some(db) = database.as_ref() {
                        persist_ws_1m_candle(db, &config.market.symbol, kline.candle).await;
                    }
                }
                if let Some(candle_ts) = hybrid_feed.ingest_ws_closed(kline.candle) {
                    if !run_evaluation_cycle(
                        &mut hybrid_feed,
                        candle_ts,
                        "ws",
                        &config,
                        limits,
                        &advisor,
                        &spot_account_id,
                        &mut execution_ctx,
                        &mut dashboard,
                        &event_tx,
                    ).await {
                        break;
                    }
                }
            }
            _ = interval.tick(), if !paused => {
                match exchange.candles(&config.market.symbol, &config.market.timeframe, config.market.candle_limit).await {
                    Ok(candles) if !candles.is_empty() => {
                        if persist_market_data_enabled() {
                            if let Some(db) = database.as_ref() {
                                if config.market.timeframe == "1m" {
                                    persist_polled_1m_window(db, &config.market.symbol, candles.clone()).await;
                                } else if !persist_timeframe_warned {
                                    persist_timeframe_warned = true;
                                    warn!(
                                        target: "persistence",
                                        timeframe = %config.market.timeframe,
                                        "PERSIST_MARKET_DATA only supports 1m timeframe"
                                    );
                                }
                            }
                        }
                        if let Some(candle_ts) = hybrid_feed.ingest_rest_window(candles) {
                            if !run_evaluation_cycle(
                                &mut hybrid_feed,
                                candle_ts,
                                "rest",
                                &config,
                                limits,
                                &advisor,
                                &spot_account_id,
                                &mut execution_ctx,
                                &mut dashboard,
                                &event_tx,
                            ).await {
                                break;
                            }
                        }
                    }
                    Ok(_) => warn!(target: "market", "Exchange returned no completed candles"),
                    Err(e) => {
                        error!(target: "market", error=%e, "Market data poll failed");
                        let _ = BotError::AmbiguousOrder(e.to_string());
                        dashboard.set_error(e.to_string());
                        let _ = event_tx.send(AppEvent::Refresh(dashboard.clone())).await;
                    }
                }
            }
        }
    }
    drop(command_tx);
    ws_shutdown.cancel();
    if let Some(handle) = ws_stream_task {
        let _ = handle.await;
    }
    if let Err(e) = ui_task.await {
        warn!(target: "ui", error=%e, "TUI task ended with an error");
    }
    info!(target: "system", environment=%config.environment, "Shutdown complete; no live-order action was performed");
    Ok(())
}
