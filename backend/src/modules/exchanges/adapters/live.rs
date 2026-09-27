use std::{
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::{
    mpsc::{self, error::TrySendError},
    Notify,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::core::config::Environment;
use crate::modules::market::Candle;

use crate::modules::exchanges::{
    registry::AccountRegistration,
    router::default_live_streams,
    ws::{WsConfig, WsSessionPlan},
    ExchangeError, MarketType,
};

/// Closed 1m kline from the websocket feed (read-only; not used for order submission).
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedKline {
    pub symbol: String,
    pub candle: Candle,
}

static DROPPED_CLOSED_KLINES: AtomicU64 = AtomicU64::new(0);

pub struct WsOverflowSignal {
    earliest: AtomicI64,
    disconnected: AtomicBool,
    changed: Notify,
}

impl WsOverflowSignal {
    pub fn new() -> Self {
        Self {
            earliest: AtomicI64::new(i64::MAX),
            disconnected: AtomicBool::new(false),
            changed: Notify::new(),
        }
    }
    pub fn record(&self, timestamp: i64) {
        self.earliest.fetch_min(timestamp, Ordering::Relaxed);
        self.changed.notify_one();
    }
    pub fn take(&self) -> Option<i64> {
        let timestamp = self.earliest.swap(i64::MAX, Ordering::Relaxed);
        (timestamp != i64::MAX).then_some(timestamp)
    }
    pub fn record_disconnect(&self) {
        self.disconnected.store(true, Ordering::Relaxed);
        self.changed.notify_one();
    }
    pub fn take_disconnect(&self) -> bool {
        self.disconnected.swap(false, Ordering::Relaxed)
    }
    pub async fn notified(&self) {
        self.changed.notified().await;
    }
}

/// Builds a validated websocket session plan for read-only market streams (no orders).
pub fn market_stream_plan(
    account: &AccountRegistration,
    symbol: &str,
) -> Result<WsSessionPlan, ExchangeError> {
    if account.id.environment != Environment::Dev
        || account.id.market != MarketType::Spot
        || !account.symbols.iter().any(|listed| listed == symbol)
    {
        return Err(ExchangeError::StreamDisconnected);
    }
    let subscriptions = default_live_streams(&account.id, symbol);
    if subscriptions.is_empty() {
        return Err(ExchangeError::StreamDisconnected);
    }
    let endpoint = binance_ws_endpoint(account, symbol)?;
    let plan = WsSessionPlan {
        endpoint,
        config: WsConfig::default(),
        subscriptions,
    };
    plan.validate()
        .map_err(|_| ExchangeError::StreamDisconnected)?;
    Ok(plan)
}

fn binance_ws_endpoint(
    account: &AccountRegistration,
    symbol: &str,
) -> Result<String, ExchangeError> {
    let Some(base @ "wss://testnet.binance.vision/ws") = account.stream_base_url.as_deref() else {
        return Err(ExchangeError::InvalidEndpoint);
    };
    let stream_symbol = symbol.replace('/', "").to_ascii_lowercase();
    if stream_symbol.is_empty() {
        return Err(ExchangeError::StreamDisconnected);
    }
    Ok(format!("{base}/{stream_symbol}@kline_1m"))
}

/// Background task: Binance kline websocket (read-only). Forwards closed candles on `closed_tx`
/// when provided. Stops when `shutdown` is cancelled (cooperative; no order path).
pub fn spawn_market_kline_stream(
    plan: WsSessionPlan,
    shutdown: CancellationToken,
    closed_tx: Option<mpsc::Sender<ClosedKline>>,
    overflow_signal: Option<Arc<WsOverflowSignal>>,
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
            match run_kline_session(&plan, &shutdown, &closed_tx, &overflow_signal).await {
                Ok(()) => {
                    debug!(target: "exchanges::ws", "Websocket session ended cleanly");
                    break;
                }
                Err(error) => {
                    if let Some(losses) = &overflow_signal {
                        losses.record_disconnect();
                    }
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
    overflow_signal: &Option<Arc<WsOverflowSignal>>,
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
                        if !forward_closed_kline(&text, &symbol_hint, closed_tx, overflow_signal).await {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn forward_closed_kline(
    text: &str,
    symbol_hint: &str,
    closed_tx: &Option<mpsc::Sender<ClosedKline>>,
    overflow_signal: &Option<Arc<WsOverflowSignal>>,
) -> bool {
    let Some(closed) = parse_closed_kline(text, symbol_hint) else {
        debug!(target: "exchanges::ws", "Websocket message ignored: not a valid closed 1m kline for configured symbol");
        return true;
    };
    info!(
        target: "exchanges::ws",
        symbol = %closed.symbol,
        close = closed.candle.close,
        ts_ms = closed.candle.timestamp_ms,
        "Closed 1m kline from websocket"
    );
    if let Some(tx) = closed_tx {
        match tx.try_send(closed) {
            Ok(()) => {}
            Err(TrySendError::Full(closed)) => {
                if let Some(losses) = overflow_signal {
                    losses.record(closed.candle.timestamp_ms);
                }
                let dropped = DROPPED_CLOSED_KLINES.fetch_add(1, Ordering::Relaxed) + 1;
                warn!(
                    target: "exchanges::ws",
                    symbol = %closed.symbol,
                    ts_ms = closed.candle.timestamp_ms,
                    channel_capacity = tx.max_capacity(),
                    dropped_closed_klines = dropped,
                    "Closed kline dropped: websocket channel full; REST poll is recovery path"
                );
            }
            Err(TrySendError::Closed(_)) => {
                debug!(target: "exchanges::ws", "Closed kline receiver dropped");
                return false;
            }
        }
    }
    true
}

fn parse_closed_kline(text: &str, symbol_hint: &str) -> Option<ClosedKline> {
    let event: BinanceKlineEvent = serde_json::from_str(text).ok()?;
    let expected = symbol_hint.replace('/', "").to_ascii_uppercase();
    if event.event_type != "kline"
        || !event.k.is_closed
        || event.k.interval != "1m"
        || expected.is_empty()
        || event.symbol.to_ascii_uppercase() != expected
        || event.k.symbol.to_ascii_uppercase() != expected
    {
        return None;
    }
    let open = event.k.o.parse::<f64>().ok()?;
    let high = event.k.h.parse::<f64>().ok()?;
    let low = event.k.l.parse::<f64>().ok()?;
    let close = event.k.c.parse::<f64>().ok()?;
    let volume = event.k.v.parse::<f64>().ok()?;
    let candle = Candle {
        timestamp_ms: event.k.open_time_ms,
        open,
        high,
        low,
        close,
        volume,
    };
    candle.validate().ok()?;
    Some(ClosedKline {
        symbol: symbol_hint.to_owned(),
        candle,
    })
}

#[derive(Debug, Deserialize)]
struct BinanceKlineEvent {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "s")]
    symbol: String,
    k: BinanceKlinePayload,
}

#[derive(Debug, Deserialize)]
struct BinanceKlinePayload {
    #[serde(rename = "t")]
    open_time_ms: i64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "i")]
    interval: String,
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
    use crate::core::config::Environment;
    use crate::modules::exchanges::bootstrap::{load_registry, spot_account_for_symbol};

    #[test]
    fn market_stream_plan_uses_secure_testnet_endpoint_for_dev() {
        let registry = load_registry(Environment::Dev).unwrap();
        let account = spot_account_for_symbol(&registry, Environment::Dev, "BTC/USDT").unwrap();
        let plan = market_stream_plan(account, "BTC/USDT").unwrap();
        assert_eq!(
            plan.endpoint,
            "wss://testnet.binance.vision/ws/btcusdt@kline_1m"
        );
        assert!(!plan.subscriptions.is_empty());
    }

    #[test]
    fn market_stream_plan_rejects_untrusted_registered_base() {
        let registry = load_registry(Environment::Dev).unwrap();
        let mut account = spot_account_for_symbol(&registry, Environment::Dev, "BTC/USDT")
            .unwrap()
            .clone();
        account.stream_base_url = Some("wss://evil.test/ws".into());
        assert!(market_stream_plan(&account, "BTC/USDT").is_err());
    }

    #[test]
    fn parse_closed_kline_ignores_open_candles() {
        let open = r#"{"e":"kline","E":1,"s":"BTCUSDT","k":{"t":60000,"s":"BTCUSDT","i":"1m","o":"1","h":"2","l":"0.5","c":"1.5","v":"10","x":false}}"#;
        assert!(parse_closed_kline(open, "BTC/USDT").is_none());
    }

    #[test]
    fn parse_closed_kline_extracts_fields() {
        let closed = r#"{"e":"kline","E":1,"s":"BTCUSDT","k":{"t":120000,"s":"BTCUSDT","i":"1m","o":"100","h":"101","l":"99","c":"100.5","v":"12.3","x":true}}"#;
        let kline = parse_closed_kline(closed, "BTC/USDT").unwrap();
        assert_eq!(kline.symbol, "BTC/USDT");
        assert_eq!(kline.candle.timestamp_ms, 120_000);
        assert_eq!(kline.candle.close, 100.5);
        assert_eq!(
            parse_closed_kline(closed, "btc/usdt").unwrap().symbol,
            "btc/usdt"
        );
    }

    fn valid_closed_payload() -> serde_json::Value {
        serde_json::json!({
            "e": "kline", "s": "BTCUSDT",
            "k": {"t": 120_000, "s": "BTCUSDT", "i": "1m", "o": "100", "h": "101", "l": "99", "c": "100.5", "v": "12.3", "x": true}
        })
    }

    #[test]
    fn parse_closed_kline_rejects_wrong_interval_symbol_and_timestamp() {
        let mut variants = Vec::new();
        let mut wrong_interval = valid_closed_payload();
        wrong_interval["k"]["i"] = "5m".into();
        variants.push(wrong_interval);
        let mut wrong_symbol = valid_closed_payload();
        wrong_symbol["k"]["s"] = "ETHUSDT".into();
        variants.push(wrong_symbol);
        let mut wrong_outer_symbol = valid_closed_payload();
        wrong_outer_symbol["s"] = "ETHUSDT".into();
        variants.push(wrong_outer_symbol);
        let mut misaligned = valid_closed_payload();
        misaligned["k"]["t"] = 120_001.into();
        variants.push(misaligned);
        let mut negative = valid_closed_payload();
        negative["k"]["t"] = (-60_000).into();
        variants.push(negative);
        for payload in variants {
            assert!(parse_closed_kline(&payload.to_string(), "BTC/USDT").is_none());
        }
    }

    #[test]
    fn parse_closed_kline_rejects_nonfinite_prices_volume_and_ohlc_incoherence() {
        for (field, value) in [
            ("o", "NaN"),
            ("h", "inf"),
            ("v", "-1"),
            ("c", "0"),
            ("h", "99"),
            ("l", "101"),
        ] {
            let mut payload = valid_closed_payload();
            payload["k"][field] = value.into();
            assert!(
                parse_closed_kline(&payload.to_string(), "BTC/USDT").is_none(),
                "field {field} value {value}"
            );
        }
    }

    #[tokio::test]
    async fn websocket_path_forwards_only_valid_closed_kline() {
        let (tx, mut rx) = mpsc::channel(2);
        let mut invalid = valid_closed_payload();
        invalid["k"]["v"] = "-1".into();
        assert!(
            forward_closed_kline(&invalid.to_string(), "BTC/USDT", &Some(tx.clone()), &None).await
        );
        assert!(rx.try_recv().is_err());

        assert!(
            forward_closed_kline(
                &valid_closed_payload().to_string(),
                "BTC/USDT",
                &Some(tx),
                &None
            )
            .await
        );
        assert_eq!(rx.try_recv().unwrap().candle.timestamp_ms, 120_000);
    }

    #[tokio::test]
    async fn saturated_closed_kline_channel_does_not_block_websocket_producer() {
        let (tx, mut rx) = mpsc::channel(1);
        let losses = Arc::new(WsOverflowSignal::new());
        let first = valid_closed_payload().to_string();
        assert!(
            forward_closed_kline(&first, "BTC/USDT", &Some(tx.clone()), &Some(losses.clone()))
                .await
        );

        let mut second = valid_closed_payload();
        second["k"]["t"] = 180_000.into();
        let forwarded = tokio::time::timeout(
            Duration::from_millis(100),
            forward_closed_kline(
                &second.to_string(),
                "BTC/USDT",
                &Some(tx),
                &Some(losses.clone()),
            ),
        )
        .await;

        assert_eq!(forwarded, Ok(true));
        assert_eq!(rx.try_recv().unwrap().candle.timestamp_ms, 120_000);
        assert!(rx.try_recv().is_err());
        assert_eq!(losses.take(), Some(180_000));
    }

    #[tokio::test]
    async fn overflow_signal_keeps_earliest_timestamp_during_burst() {
        let (tx, mut rx) = mpsc::channel(1);
        let losses = Arc::new(WsOverflowSignal::new());
        let first = valid_closed_payload().to_string();
        assert!(
            forward_closed_kline(&first, "BTC/USDT", &Some(tx.clone()), &Some(losses.clone()))
                .await
        );
        for timestamp in [240_000, 180_000, 300_000] {
            let mut payload = valid_closed_payload();
            payload["k"]["t"] = timestamp.into();
            assert!(
                forward_closed_kline(
                    &payload.to_string(),
                    "BTC/USDT",
                    &Some(tx.clone()),
                    &Some(losses.clone())
                )
                .await
            );
        }
        assert_eq!(losses.take(), Some(180_000));
        assert_eq!(losses.take(), None);
        assert_eq!(rx.try_recv().unwrap().candle.timestamp_ms, 120_000);
        losses.record_disconnect();
        assert!(losses.take_disconnect());
        assert!(!losses.take_disconnect());
    }

    #[tokio::test]
    async fn closed_receiver_stops_valid_forwarding_without_promoting_invalid_messages() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let mut invalid = valid_closed_payload();
        invalid["k"]["x"] = false.into();

        assert!(
            forward_closed_kline(&invalid.to_string(), "BTC/USDT", &Some(tx.clone()), &None).await
        );
        assert!(
            !forward_closed_kline(
                &valid_closed_payload().to_string(),
                "BTC/USDT",
                &Some(tx),
                &None
            )
            .await
        );
    }
}
