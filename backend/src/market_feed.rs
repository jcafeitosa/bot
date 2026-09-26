//! Hybrid REST + websocket candle buffer for live monitor evaluation.
//!
//! REST polls refresh the full sliding window (backfill / fallback). Closed 1m websocket
//! klines upsert into the same series and can trigger evaluation without waiting for the poll.

use mantis_ta::types::Candle;

use crate::market::Candle as MarketCandle;

/// Websocket feed is fixed at 1m klines (`exchanges::live`).
pub fn ws_matches_configured_timeframe(timeframe: &str) -> bool {
    timeframe == "1m"
}

#[derive(Debug, Clone)]
pub struct HybridCandleFeed {
    candles: Vec<Candle>,
    limit: usize,
    last_evaluated_ts: Option<i64>,
}

impl HybridCandleFeed {
    pub fn new(limit: u32) -> Self {
        let limit = usize::try_from(limit).unwrap_or(1).max(1);
        Self {
            candles: Vec::new(),
            limit,
            last_evaluated_ts: None,
        }
    }

    pub fn candles(&self) -> &[Candle] {
        &self.candles
    }

    pub fn last_evaluated_ts(&self) -> Option<i64> {
        self.last_evaluated_ts
    }

    pub fn mark_evaluated(&mut self, candle_timestamp_ms: i64) {
        self.last_evaluated_ts = Some(candle_timestamp_ms);
    }

    /// Ingest REST poll window. Returns timestamp to evaluate when the newest bar was not yet evaluated.
    pub fn ingest_rest_window(&mut self, candles: Vec<Candle>) -> Option<i64> {
        if candles.is_empty() {
            return None;
        }
        self.candles = sort_dedup(candles);
        trim_tail(&mut self.candles, self.limit);
        self.latest_trigger_ts()
    }

    /// Upsert one closed websocket 1m bar. Returns timestamp to evaluate when it advances the series.
    pub fn ingest_ws_closed(&mut self, candle: MarketCandle) -> Option<i64> {
        let bar = candle.to_mantis();
        let ts = bar.timestamp;
        upsert_bar(&mut self.candles, bar);
        trim_tail(&mut self.candles, self.limit);
        if self.should_trigger_eval(ts) {
            Some(ts)
        } else {
            None
        }
    }

    fn latest_trigger_ts(&mut self) -> Option<i64> {
        let ts = self.candles.last().map(|c| c.timestamp)?;
        if self.should_trigger_eval(ts) {
            Some(ts)
        } else {
            None
        }
    }

    fn should_trigger_eval(&self, candle_timestamp_ms: i64) -> bool {
        match self.last_evaluated_ts {
            None => true,
            Some(last) => candle_timestamp_ms > last,
        }
    }
}

fn sort_dedup(candles: Vec<Candle>) -> Vec<Candle> {
    let mut out = candles;
    out.sort_by_key(|c| c.timestamp);
    out.dedup_by_key(|c| c.timestamp);
    out
}

fn upsert_bar(series: &mut Vec<Candle>, bar: Candle) {
    if let Some(idx) = series.iter().position(|c| c.timestamp == bar.timestamp) {
        series[idx] = bar;
    } else {
        series.push(bar);
        series.sort_by_key(|c| c.timestamp);
    }
}

fn trim_tail(series: &mut Vec<Candle>, limit: usize) {
    if series.len() > limit {
        let drop = series.len() - limit;
        series.drain(0..drop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(ts: i64, close: f64) -> Candle {
        Candle {
            timestamp: ts,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn rest_ingest_triggers_once_per_timestamp() {
        let mut feed = HybridCandleFeed::new(10);
        let window = vec![bar(0, 1.0), bar(60_000, 2.0)];
        assert_eq!(feed.ingest_rest_window(window.clone()), Some(60_000));
        feed.mark_evaluated(60_000);
        assert_eq!(feed.ingest_rest_window(window), None);
    }

    #[test]
    fn ws_upsert_replaces_same_timestamp_and_triggers_new_bar() {
        let mut feed = HybridCandleFeed::new(10);
        feed.ingest_rest_window(vec![bar(0, 1.0), bar(60_000, 2.0)]);
        feed.mark_evaluated(60_000);

        let updated = MarketCandle {
            timestamp_ms: 60_000,
            open: 2.0,
            high: 3.0,
            low: 1.5,
            close: 2.5,
            volume: 1.0,
        };
        assert_eq!(feed.ingest_ws_closed(updated), None);

        let next = MarketCandle {
            timestamp_ms: 120_000,
            open: 2.5,
            high: 3.0,
            low: 2.0,
            close: 2.8,
            volume: 1.0,
        };
        assert_eq!(feed.ingest_ws_closed(next), Some(120_000));
        assert_eq!(feed.candles().last().unwrap().close, 2.8);
    }

    #[test]
    fn ws_timeframe_gate() {
        assert!(ws_matches_configured_timeframe("1m"));
        assert!(!ws_matches_configured_timeframe("15m"));
    }
}
