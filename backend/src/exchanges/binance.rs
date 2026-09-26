use async_trait::async_trait;
use ccxt_core::types::ohlcv_request::OhlcvRequest;
use ccxt_exchanges::binance::{Binance, BinanceBuilder};
use mantis_ta::types::Candle;

use super::MarketDataSource;
use crate::{
    config::{Credentials, Environment},
    error::{BotError, BotResult},
};

pub struct BinanceMarketData {
    exchange: Binance,
}

impl BinanceMarketData {
    pub fn new(credentials: Credentials, environment: Environment) -> BotResult<Self> {
        let mut builder = BinanceBuilder::new()
            .sandbox(environment == Environment::Dev)
            .default_type("spot")
            .timeout_secs(15)
            .enable_rate_limit(true);
        if let Some(key) = credentials.api_key {
            builder = builder.api_key(key);
        }
        if let Some(secret) = credentials.secret {
            builder = builder.secret(secret);
        }
        let exchange = builder.build().map_err(|e| {
            BotError::Exchange(format!(
                "cannot configure Binance {environment} adapter: {e}"
            ))
        })?;
        Ok(Self { exchange })
    }
}

#[async_trait]
impl MarketDataSource for BinanceMarketData {
    async fn candles(&self, symbol: &str, timeframe: &str, limit: u32) -> BotResult<Vec<Candle>> {
        let request = OhlcvRequest::builder()
            .symbol(symbol)
            .timeframe(timeframe)
            .limit(limit)
            .build()
            .map_err(|e| BotError::MarketData(format!("invalid candle request: {e}")))?;
        let rows = self
            .exchange
            .fetch_ohlcv_v2(request)
            .await
            .map_err(|e| BotError::MarketData(format!("Binance OHLCV request failed: {e}")))?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let period_ms = timeframe_millis(timeframe)
            .ok_or_else(|| BotError::Configuration("unsupported candle interval".into()))?;
        let mut candles = rows
            .into_iter()
            .filter(|r| r.timestamp.saturating_add(period_ms) <= now_ms)
            .map(|r| Candle {
                timestamp: r.timestamp,
                open: r.open,
                high: r.high,
                low: r.low,
                close: r.close,
                volume: r.volume,
            })
            .collect::<Vec<_>>();
        candles.sort_by_key(|c| c.timestamp);
        candles.dedup_by_key(|c| c.timestamp);
        if candles.iter().any(|c| {
            !c.close.is_finite()
                || c.close <= 0.0
                || !c.high.is_finite()
                || !c.low.is_finite()
                || c.low > c.high
        }) {
            return Err(BotError::MarketData(
                "exchange returned invalid candle values".into(),
            ));
        }
        Ok(candles)
    }
}

fn timeframe_millis(tf: &str) -> Option<i64> {
    if let Some(minutes) = tf.strip_suffix('m') {
        return minutes.parse::<i64>().ok()?.checked_mul(60_000);
    }
    if let Some(hours) = tf.strip_suffix('h') {
        return hours.parse::<i64>().ok()?.checked_mul(3_600_000);
    }
    None
}
