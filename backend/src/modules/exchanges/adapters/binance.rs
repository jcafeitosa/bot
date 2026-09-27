use async_trait::async_trait;
use ccxt_core::types::ohlcv_request::OhlcvRequest;
use ccxt_core::types::OHLCV;
use ccxt_exchanges::binance::{Binance, BinanceBuilder};
use mantis_ta::types::Candle;

use super::account_file::require_exact_spot_endpoint;
use crate::modules::exchanges::{
    market_data::MarketDataSource, registry::AccountRegistration, MarketType,
};
use crate::{
    core::config::{Credentials, Environment},
    core::error::{BotError, BotResult},
    modules::market::Candle as MarketCandle,
};

pub struct BinanceMarketData {
    exchange: Binance,
}

impl BinanceMarketData {
    pub fn new(credentials: Credentials, account: &AccountRegistration) -> BotResult<Self> {
        if account.id.environment != Environment::Dev || account.id.market != MarketType::Spot {
            return Err(BotError::Configuration(
                "Binance market data requires a dev Spot account".into(),
            ));
        }
        let rest_base = account
            .rest_base_url
            .as_deref()
            .ok_or_else(|| BotError::Configuration("missing Spot REST endpoint".into()))?;
        require_exact_spot_endpoint(Some(rest_base), "https", "/")
            .map_err(|e| BotError::Configuration(e.to_string()))?;
        let origin = reqwest::Url::parse(rest_base)
            .map_err(|e| BotError::Configuration(format!("invalid Spot REST endpoint: {e}")))?
            .origin()
            .ascii_serialization();
        let public_url = format!("{origin}/api/v3");
        let mut builder = BinanceBuilder::new()
            .sandbox(true)
            .default_type("spot")
            .timeout_secs(15)
            .enable_rate_limit(true);
        if let Some(key) = credentials.api_key {
            builder = builder.api_key(key);
        }
        if let Some(secret) = credentials.secret {
            builder = builder.secret(secret);
        }
        let mut exchange = builder.build().map_err(|e| {
            BotError::Exchange(format!(
                "cannot configure Binance {} adapter: {e}",
                account.id.environment
            ))
        })?;
        exchange
            .base_mut()
            .config
            .url_overrides
            .insert("public".into(), public_url.clone());
        if exchange.get_rest_url_public() != public_url {
            return Err(BotError::Configuration(
                "Binance public REST endpoint does not match configured testnet origin".into(),
            ));
        }
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
        completed_candles(rows, timeframe, now_ms)
    }
}

fn completed_candles(rows: Vec<OHLCV>, timeframe: &str, now_ms: i64) -> BotResult<Vec<Candle>> {
    let period_ms = timeframe_millis(timeframe)
        .ok_or_else(|| BotError::Configuration("unsupported candle interval".into()))?;
    let mut candles = rows
        .into_iter()
        .filter(|r| r.timestamp.saturating_add(period_ms) <= now_ms)
        .map(|r| {
            let bar = MarketCandle {
                timestamp_ms: r.timestamp,
                open: r.open,
                high: r.high,
                low: r.low,
                close: r.close,
                volume: r.volume,
            };
            bar.validate().map_err(|error| {
                BotError::MarketData(format!("exchange returned invalid candle: {error}"))
            })?;
            if bar.timestamp_ms % period_ms != 0 {
                return Err(BotError::MarketData(format!(
                    "exchange returned candle misaligned to {timeframe}: {}",
                    bar.timestamp_ms
                )));
            }
            Ok(bar.to_mantis())
        })
        .collect::<BotResult<Vec<_>>>()?;
    candles.sort_by_key(|c| c.timestamp);
    if candles
        .windows(2)
        .any(|pair| pair[0].timestamp == pair[1].timestamp)
    {
        return Err(BotError::MarketData(
            "exchange returned duplicate candle timestamps".into(),
        ));
    }
    Ok(candles)
}

fn timeframe_millis(tf: &str) -> Option<i64> {
    if let Some(minutes) = tf.strip_suffix('m') {
        return minutes
            .parse::<i64>()
            .ok()?
            .checked_mul(60_000)
            .filter(|ms| *ms > 0);
    }
    if let Some(hours) = tf.strip_suffix('h') {
        return hours
            .parse::<i64>()
            .ok()?
            .checked_mul(3_600_000)
            .filter(|ms| *ms > 0);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{completed_candles, BinanceMarketData};
    use crate::core::config::{Credentials, Environment};
    use crate::modules::exchanges::bootstrap::{load_registry, spot_account_for_symbol};
    use ccxt_core::types::OHLCV;

    #[test]
    fn public_rest_endpoint_uses_registered_spot_testnet_origin() {
        let registry = load_registry(Environment::Dev).unwrap();
        let account = spot_account_for_symbol(&registry, Environment::Dev, "BTC/USDT").unwrap();
        let credentials = Credentials {
            api_key: None,
            secret: None,
        };
        let market = BinanceMarketData::new(credentials, account).unwrap();
        assert_eq!(
            market.exchange.get_rest_url_public(),
            "https://testnet.binance.vision/api/v3"
        );
    }

    #[test]
    fn public_rest_adapter_rejects_untrusted_origin_even_with_credentials() {
        let registry = load_registry(Environment::Dev).unwrap();
        let mut account = spot_account_for_symbol(&registry, Environment::Dev, "BTC/USDT")
            .unwrap()
            .clone();
        account.rest_base_url = Some("https://api.binance.com".into());
        let credentials = Credentials {
            api_key: Some("test-key".into()),
            secret: Some("test-secret".into()),
        };
        assert!(BinanceMarketData::new(credentials, &account).is_err());
    }

    fn bar(timestamp: i64) -> OHLCV {
        OHLCV {
            timestamp,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 10.0,
        }
    }

    #[test]
    fn completed_rest_window_rejects_invalid_middle_bar_whole() {
        let mut invalid = bar(60_000);
        invalid.volume = -1.0;
        let rows = vec![bar(0), invalid, bar(120_000)];
        assert!(completed_candles(rows, "1m", 180_000).is_err());
    }

    #[test]
    fn completed_rest_window_returns_valid_bars_and_omits_unfinished_bar() {
        let rows = vec![bar(60_000), bar(0), bar(120_000)];
        let candles = completed_candles(rows, "1m", 120_000).unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].timestamp, 0);
        assert_eq!(candles[1].timestamp, 60_000);
    }

    #[test]
    fn completed_rest_window_rejects_conflicting_duplicate_timestamps() {
        let first = bar(60_000);
        let mut conflicting = bar(60_000);
        conflicting.close = 100.5;
        assert!(completed_candles(vec![bar(0), first, conflicting], "1m", 120_000).is_err());
    }

    #[test]
    fn completed_rest_window_rejects_misalignment_nonfinite_and_incoherent_ohlcv() {
        let mut invalid_cases = Vec::new();
        let mut misaligned = bar(60_000);
        misaligned.timestamp = 60_001;
        invalid_cases.push(misaligned);
        let mut negative_timestamp = bar(60_000);
        negative_timestamp.timestamp = -60_000;
        invalid_cases.push(negative_timestamp);
        let mut nan_open = bar(60_000);
        nan_open.open = f64::NAN;
        invalid_cases.push(nan_open);
        let mut infinite_volume = bar(60_000);
        infinite_volume.volume = f64::INFINITY;
        invalid_cases.push(infinite_volume);
        let mut zero_close = bar(60_000);
        zero_close.close = 0.0;
        invalid_cases.push(zero_close);
        let mut high_below_open = bar(60_000);
        high_below_open.high = 99.5;
        invalid_cases.push(high_below_open);
        let mut low_above_close = bar(60_000);
        low_above_close.low = 100.5;
        invalid_cases.push(low_above_close);

        for invalid in invalid_cases {
            assert!(completed_candles(vec![bar(0), invalid, bar(120_000)], "1m", 180_000).is_err());
        }
        assert!(completed_candles(vec![bar(60_000)], "15m", 960_000).is_err());
    }

    #[test]
    fn invalid_zero_timeframe_returns_error() {
        assert!(completed_candles(vec![bar(0)], "0m", 60_000).is_err());
    }
}
