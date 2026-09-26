use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const BASE_TIMEFRAME_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timeframe {
    minutes: u32,
}

impl Timeframe {
    pub fn new(minutes: u32) -> Result<Self, MarketError> {
        if ![1, 3, 5, 15, 30, 60, 240].contains(&minutes) {
            return Err(MarketError::UnsupportedTimeframe(minutes));
        }
        Ok(Self { minutes })
    }
    pub fn minutes(self) -> u32 {
        self.minutes
    }
    pub fn duration_ms(self) -> i64 {
        i64::from(self.minutes) * BASE_TIMEFRAME_MS
    }
    pub fn as_str(self) -> &'static str {
        match self.minutes {
            1 => "1m",
            3 => "3m",
            5 => "5m",
            15 => "15m",
            30 => "30m",
            60 => "1h",
            240 => "4h",
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Candle {
    pub fn to_mantis(self) -> mantis_ta::types::Candle {
        mantis_ta::types::Candle {
            timestamp: self.timestamp_ms,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), MarketError> {
        if self.timestamp_ms < 0
            || [self.open, self.high, self.low, self.close, self.volume]
                .iter()
                .any(|v| !v.is_finite())
            || self.open <= 0.0
            || self.high <= 0.0
            || self.low <= 0.0
            || self.close <= 0.0
            || self.volume < 0.0
            || self.high < self.open.max(self.close)
            || self.low > self.open.min(self.close)
            || self.high < self.low
        {
            return Err(MarketError::InvalidCandle(self.timestamp_ms));
        }
        if self.timestamp_ms % BASE_TIMEFRAME_MS != 0 {
            return Err(MarketError::UnalignedTimestamp(self.timestamp_ms));
        }
        Ok(())
    }
}

impl From<mantis_ta::types::Candle> for Candle {
    fn from(candle: mantis_ta::types::Candle) -> Self {
        Self {
            timestamp_ms: candle.timestamp,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            volume: candle.volume,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub dataset_id: String,
    pub symbol: String,
    pub base_timeframe: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub candle_count: usize,
    pub gap_count: usize,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataset {
    pub manifest: DatasetManifest,
    pub candles: Vec<Candle>,
}

impl HistoricalDataset {
    pub fn from_1m(
        symbol: impl Into<String>,
        source: impl Into<String>,
        candles: Vec<Candle>,
    ) -> Result<Self, MarketError> {
        let symbol = symbol.into().trim().to_ascii_uppercase();
        if !symbol.contains('/') {
            return Err(MarketError::InvalidSymbol);
        }
        if candles.is_empty() {
            return Err(MarketError::EmptyDataset);
        }
        for candle in &candles {
            candle.validate()?;
        }
        for pair in candles.windows(2) {
            if pair[1].timestamp_ms <= pair[0].timestamp_ms {
                return Err(MarketError::NonMonotonic);
            }
        }
        let gaps = candles
            .windows(2)
            .filter(|p| p[1].timestamp_ms - p[0].timestamp_ms != BASE_TIMEFRAME_MS)
            .count();
        let manifest = DatasetManifest {
            dataset_id: String::new(),
            symbol,
            base_timeframe: "1m".into(),
            start_ms: candles[0].timestamp_ms,
            end_ms: candles.last().unwrap().timestamp_ms + BASE_TIMEFRAME_MS,
            candle_count: candles.len(),
            gap_count: gaps,
            source: source.into(),
        };
        let mut dataset = Self { manifest, candles };
        dataset.manifest.dataset_id = dataset.fingerprint();
        Ok(dataset)
    }

    pub fn from_mantis_1m(
        symbol: impl Into<String>,
        source: impl Into<String>,
        candles: Vec<mantis_ta::types::Candle>,
    ) -> Result<Self, MarketError> {
        let market_candles = candles.into_iter().map(Candle::from).collect();
        Self::from_1m(symbol, source, market_candles)
    }

    pub fn fingerprint(&self) -> String {
        // Stable FNV-1a fingerprint of normalized manifest and IEEE-754 data; not cryptographic.
        let mut hash: u64 = 0xcbf29ce484222325;
        let mut feed = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        };
        feed(self.manifest.symbol.as_bytes());
        feed(self.manifest.base_timeframe.as_bytes());
        for c in &self.candles {
            feed(&c.timestamp_ms.to_le_bytes());
            feed(&c.open.to_bits().to_le_bytes());
            feed(&c.high.to_bits().to_le_bytes());
            feed(&c.low.to_bits().to_le_bytes());
            feed(&c.close.to_bits().to_le_bytes());
            feed(&c.volume.to_bits().to_le_bytes());
        }
        format!("fnv1a64:{hash:016x}")
    }

    pub fn resample(&self, timeframe: Timeframe) -> Result<Vec<Candle>, MarketError> {
        if timeframe.minutes == 1 {
            return Ok(self.candles.clone());
        }
        let width = timeframe.duration_ms();
        let mut out = Vec::new();
        let mut index = 0;
        while index < self.candles.len() {
            let first = self.candles[index];
            let bucket_start = first.timestamp_ms.div_euclid(width) * width;
            if first.timestamp_ms != bucket_start {
                return Err(MarketError::IncompleteBucket(bucket_start));
            }
            let bucket_end = bucket_start + width;
            let mut j = index;
            let mut group = Vec::with_capacity(timeframe.minutes as usize);
            while j < self.candles.len() && self.candles[j].timestamp_ms < bucket_end {
                if self.candles[j].timestamp_ms
                    != bucket_start + (group.len() as i64 * BASE_TIMEFRAME_MS)
                {
                    return Err(MarketError::Gap(
                        bucket_start + (group.len() as i64 * BASE_TIMEFRAME_MS),
                    ));
                }
                group.push(self.candles[j]);
                j += 1;
            }
            if group.len() != timeframe.minutes as usize {
                return Err(MarketError::IncompleteBucket(bucket_start));
            }
            out.push(Candle {
                timestamp_ms: bucket_start,
                open: group[0].open,
                high: group
                    .iter()
                    .map(|c| c.high)
                    .fold(f64::NEG_INFINITY, f64::max),
                low: group.iter().map(|c| c.low).fold(f64::INFINITY, f64::min),
                close: group.last().unwrap().close,
                volume: group.iter().map(|c| c.volume).sum(),
            });
            index = j;
        }
        Ok(out)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MarketError {
    #[error("unsupported timeframe {0} minutes")]
    UnsupportedTimeframe(u32),
    #[error("invalid candle at timestamp {0}")]
    InvalidCandle(i64),
    #[error("candle timestamp {0} is not aligned to 1m boundary")]
    UnalignedTimestamp(i64),
    #[error("dataset has no candles")]
    EmptyDataset,
    #[error("candle timestamps must be strictly increasing and unique")]
    NonMonotonic,
    #[error("invalid unified symbol")]
    InvalidSymbol,
    #[error("missing 1m candle at timestamp {0}")]
    Gap(i64),
    #[error("partial or misaligned {0}m aggregate bucket")]
    IncompleteBucket(i64),
}

#[cfg(test)]
mod tests {
    use super::*;
    fn candle(ts: i64, close: f64) -> Candle {
        Candle {
            timestamp_ms: ts,
            open: close,
            high: close + 1.,
            low: close - 1.,
            close,
            volume: 2.,
        }
    }
    #[test]
    fn aggregates_1m_bars_with_ohlcv_semantics() {
        let bars = vec![candle(0, 10.), candle(60_000, 11.), candle(120_000, 9.)];
        let data = HistoricalDataset::from_1m("BTC/USDT", "fixture", bars).unwrap();
        let result = data.resample(Timeframe::new(3).unwrap()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].open, 10.);
        assert_eq!(result[0].close, 9.);
        assert_eq!(result[0].high, 12.);
        assert_eq!(result[0].low, 8.);
        assert_eq!(result[0].volume, 6.);
    }
    #[test]
    fn refuses_partial_aggregated_bars() {
        let data = HistoricalDataset::from_1m(
            "BTC/USDT",
            "fixture",
            vec![candle(0, 10.), candle(60_000, 11.)],
        )
        .unwrap();
        assert!(matches!(
            data.resample(Timeframe::new(3).unwrap()),
            Err(MarketError::IncompleteBucket(0))
        ));
    }
    #[test]
    fn refuses_gaps_in_base_data() {
        let data = HistoricalDataset::from_1m(
            "BTC/USDT",
            "fixture",
            vec![candle(0, 10.), candle(120_000, 11.)],
        )
        .unwrap();
        assert!(matches!(
            data.resample(Timeframe::new(3).unwrap()),
            Err(MarketError::Gap(60_000))
        ));
    }
    #[test]
    fn mantis_round_trip_preserves_ohlcv() {
        let original = candle(120_000, 42.5);
        let restored = Candle::from(original.to_mantis());
        assert_eq!(original, restored);
    }

    #[test]
    fn rejects_duplicate_timestamp() {
        assert_eq!(
            HistoricalDataset::from_1m("BTC/USDT", "fixture", vec![candle(0, 10.), candle(0, 11.)])
                .unwrap_err(),
            MarketError::NonMonotonic
        );
    }
}
