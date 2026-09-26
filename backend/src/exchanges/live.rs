use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::config::Environment;
use crate::market::Candle;

use super::{
    router::default_live_streams,
    ws::{WsConfig, WsSessionPlan},
    ExchangeAccountId, ExchangeError,
};

/// Closed 1m kline from the websocket feed (read-only; not used for order submission).
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedKline {
    pub symbol: String,
    pub candle: Candle,
}

/// Builds a validated websocket session plan for read-only market streams (no orders).
pub fn market_stream_plan(
    account: &ExchangeAccountId,
    symbol: &str,
    environment: Environment,
) -> Result<WsSessionPlan, ExchangeError> {
    let subscriptions = default_live_streams(account, symbol);
    if subscriptions.is_empty() {
        return Err(ExchangeError::StreamDisconnected);
    }
    let endpoint = binance_ws_endpoint(environment, symbol)?;
    let plan = WsSessionPlan {
        endpoint,
        config: WsConfig::default(),
        subscriptions,
    };
    plan.validate()
        .map_err(|_| ExchangeError::StreamDisconnected)?;
    Ok(plan)
}

fn binance_ws_endpoint(environment: Environment, symbol: &str) -> Result<String, ExchangeError> {
    let stream_symbol = symbol.replace('/', "").to_ascii_lowercase();
    if stream_symbol.is_empty() {
        return Err(ExchangeError::StreamDisconnected);
    }
    let host = match environment {
        Environment::Dev => "stream.testnet.binance.vision",
        Environment::Prod => "stream.binance.com",
    };
    Ok(format!("wss://{host}/ws/{stream_symbol}@kline_1m"))
}

/// Background task: Binance kline websocket (read-only). Forwards closed candles on `closed_tx`
/// when provided. Stops when `shutdown` is cancelled (cooperative; no order path).
pub fn spawn_market_kline_stream(
    plan: WsSessionPlan,
    shutdown: CancellationToken,
    closed_tx: Option<mpsc::Sender<ClosedKline>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(error) = plan.validate() {
            warn!(target: "exchanges::ws", %error, "Websocket stream skipped: invalid plan");
            return;
        }
        let mut attempt = 0u32;
        while !shutdown.is_cancelled() {
            info!(
                target: "exchanges::ws",
                endpoint = %plan.endpoint,
                subscriptions = plan.subscriptions.len(),
                attempt,
                "Connecting to kline websocket (read-only; REST poll remains primary feed)"
            );
            match run_kline_session(&plan, &shutdown, &closed_tx).await {
                Ok(()) => {
                    debug!(target: "exchanges::ws", "Websocket session ended cleanly");
                    break;
                }
                Err(error) => {
                    warn!(target: "exchanges::ws", %error, "Websocket session failed");
                }
            }
            if shutdown.is_cancelled() {
                break;
            }
            attempt = attempt.saturating_add(1);
            let delay = plan.config.reconnect_delay(attempt);
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = tokio::time::sleep(delay) => {}
            }
        }
        info!(target: "exchanges::ws", "Kline websocket task stopped");
    })
}

async fn run_kline_session(
    plan: &WsSessionPlan,
    shutdown: &CancellationToken,
    closed_tx: &Option<mpsc::Sender<ClosedKline>>,
) -> Result<(), WsSessionError> {
    let connect = connect_async(plan.endpoint.as_str());
    let timeout = Duration::from_secs(plan.config.connect_timeout_secs);
    let (ws_stream, _) = tokio::time::timeout(timeout, connect)
        .await
        .map_err(|_| WsSessionError::ConnectTimeout)?
        .map_err(WsSessionError::Connect)?;

    let (mut write, mut read) = ws_stream.split();
    let heartbeat = Duration::from_secs(plan.config.heartbeat_secs);
    let mut ping_tick = tokio::time::interval(heartbeat);
    ping_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let symbol_hint = plan
        .subscriptions
        .first()
        .map(|s| s.symbol.clone())
        .unwrap_or_default();

    while !shutdown.is_cancelled() {
        tokio::select! {
            _ = shutdown.cancelled() => return Ok(()),
            _ = ping_tick.tick() => {
                if write.send(Message::Ping(Vec::new().into())).await.is_err() {
                    return Err(WsSessionError::Disconnected);
                }
            }
            incoming = read.next() => {
                match incoming {
                    None => return Err(WsSessionError::Disconnected),
                    Some(Err(error)) => return Err(WsSessionError::Protocol(error.to_string())),
                    Some(Ok(Message::Close(_))) => return Err(WsSessionError::Disconnected),
                    Some(Ok(Message::Ping(payload))) => {
                        if write.send(Message::Pong(payload)).await.is_err() {
                            return Err(WsSessionError::Disconnected);
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Ok(Message::Text(text))) => {
                        if let Some(closed) = parse_closed_kline(&text, &symbol_hint) {
                            info!(
                                target: "exchanges::ws",
                                symbol = %closed.symbol,
                                close = closed.candle.close,
                                ts_ms = closed.candle.timestamp_ms,
                                "Closed 1m kline from websocket"
                            );
                            if let Some(tx) = closed_tx {
                                if tx.send(closed).await.is_err() {
                                    debug!(target: "exchanges::ws", "Closed kline receiver dropped");
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_closed_kline(text: &str, symbol_hint: &str) -> Option<ClosedKline> {
    let event: BinanceKlineEvent = serde_json::from_str(text).ok()?;
    if event.event_type != "kline" || !event.k.is_closed {
        return None;
    }
    let open = event.k.o.parse::<f64>().ok()?;
    let high = event.k.h.parse::<f64>().ok()?;
    let low = event.k.l.parse::<f64>().ok()?;
    let close = event.k.c.parse::<f64>().ok()?;
    let volume = event.k.v.parse::<f64>().ok()?;
    let symbol = if symbol_hint.is_empty() {
        event.k.symbol.clone()
    } else {
        symbol_hint.to_owned()
    };
    Some(ClosedKline {
        symbol,
        candle: Candle {
            timestamp_ms: event.k.open_time_ms,
            open,
            high,
            low,
            close,
            volume,
        },
    })
}

#[derive(Debug, Deserialize)]
struct BinanceKlineEvent {
    #[serde(rename = "e")]
    event_type: String,
    k: BinanceKlinePayload,
}

#[derive(Debug, Deserialize)]
struct BinanceKlinePayload {
    #[serde(rename = "t")]
    open_time_ms: i64,
    #[serde(rename = "s")]
    symbol: String,
    o: String,
    h: String,
    l: String,
    c: String,
    v: String,
    #[serde(rename = "x")]
    is_closed: bool,
}

#[derive(Debug, thiserror::Error)]
enum WsSessionError {
    #[error("websocket connect timed out")]
    ConnectTimeout,
    #[error("websocket connect failed: {0}")]
    Connect(tokio_tungstenite::tungstenite::Error),
    #[error("websocket disconnected")]
    Disconnected,
    #[error("websocket protocol error: {0}")]
    Protocol(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;
    use crate::exchanges::{ExchangeId, MarketType};

    #[test]
    fn market_stream_plan_uses_secure_testnet_endpoint_for_dev() {
        let account = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "paper-main",
            Environment::Dev,
        )
        .unwrap();
        let plan = market_stream_plan(&account, "BTC/USDT", Environment::Dev).unwrap();
        assert!(plan.endpoint.starts_with("wss://"));
        assert!(plan.endpoint.contains("testnet"));
        assert!(!plan.subscriptions.is_empty());
    }

    #[test]
    fn parse_closed_kline_ignores_open_candles() {
        let open = r#"{"e":"kline","E":1,"s":"BTCUSDT","k":{"t":60000,"s":"BTCUSDT","o":"1","h":"2","l":"0.5","c":"1.5","v":"10","x":false}}"#;
        assert!(parse_closed_kline(open, "BTC/USDT").is_none());
    }

    #[test]
    fn parse_closed_kline_extracts_fields() {
        let closed = r#"{"e":"kline","E":1,"s":"BTCUSDT","k":{"t":120000,"s":"BTCUSDT","o":"100","h":"101","l":"99","c":"100.5","v":"12.3","x":true}}"#;
        let kline = parse_closed_kline(closed, "BTC/USDT").unwrap();
        assert_eq!(kline.symbol, "BTC/USDT");
        assert_eq!(kline.candle.timestamp_ms, 120_000);
        assert_eq!(kline.candle.close, 100.5);
    }
}
