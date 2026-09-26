use crate::config::OperationMode;
use mantis_ta::{
    indicators::{Indicator, SMA},
    types::Candle,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Warmup,
    Hold,
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategySnapshot {
    pub signal: Signal,
    pub close: f64,
    pub fast_sma: Option<f64>,
    pub slow_sma: Option<f64>,
    pub candle_timestamp_ms: i64,
}

pub fn periods_for_mode(mode: OperationMode) -> (usize, usize) {
    match mode {
        OperationMode::Hft => (1, 2),
        OperationMode::Scalper => (5, 20),
        OperationMode::DayTrader => (5, 20),
        OperationMode::SwingTrader => (20, 50),
    }
}

pub fn evaluate(candles: &[Candle], fast: usize, slow: usize) -> StrategySnapshot {
    if candles.is_empty() || fast == 0 || fast >= slow {
        return StrategySnapshot {
            signal: Signal::Warmup,
            close: candles.last().map(|c| c.close).unwrap_or_default(),
            fast_sma: None,
            slow_sma: None,
            candle_timestamp_ms: candles.last().map(|c| c.timestamp).unwrap_or_default(),
        };
    }
    let fast_values = SMA::new(fast).calculate(candles);
    let slow_values = SMA::new(slow).calculate(candles);
    let last = candles.last().expect("caller must check nonempty candles");
    let n = candles.len();
    let fast_now = fast_values[n - 1];
    let slow_now = slow_values[n - 1];
    let signal = match (
        fast_now,
        slow_now,
        n.checked_sub(2).and_then(|i| fast_values[i]),
        n.checked_sub(2).and_then(|i| slow_values[i]),
    ) {
        (Some(fast_now), Some(slow_now), Some(fast_prev), Some(slow_prev))
            if fast_prev <= slow_prev && fast_now > slow_now =>
        {
            Signal::Buy
        }
        (Some(fast_now), Some(slow_now), Some(fast_prev), Some(slow_prev))
            if fast_prev >= slow_prev && fast_now < slow_now =>
        {
            Signal::Sell
        }
        (Some(_), Some(_), Some(_), Some(_)) => Signal::Hold,
        _ => Signal::Warmup,
    };
    StrategySnapshot {
        signal,
        close: last.close,
        fast_sma: fast_now,
        slow_sma: slow_now,
        candle_timestamp_ms: last.timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bars(closes: &[f64]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(i, close)| Candle {
                timestamp: i as i64 * 60_000,
                open: *close,
                high: *close,
                low: *close,
                close: *close,
                volume: 1.0,
            })
            .collect()
    }
    #[test]
    fn waits_for_indicator_warmup() {
        let b = bars(&[1., 2., 3., 4.]);
        assert_eq!(evaluate(&b, 3, 5).signal, Signal::Warmup);
    }
    #[test]
    fn detects_fast_sma_crossing_above_slow_sma() {
        // Fast SMA(2) crosses above slow SMA(4) exactly on the final bar.
        let b = bars(&[5., 5., 5., 5., 1., 10.]);
        assert_eq!(evaluate(&b, 2, 4).signal, Signal::Buy);
    }
    #[test]
    fn detects_fast_sma_crossing_below_slow_sma() {
        // Fast SMA(2) crosses below slow SMA(4) exactly on the final bar.
        let b = bars(&[10., 10., 10., 10., 10., 1.]);
        assert_eq!(evaluate(&b, 2, 4).signal, Signal::Sell);
    }
    #[test]
    fn holds_when_both_averages_are_on_the_same_side() {
        // Both SMAs defined, but the cross happened earlier: no fresh signal.
        let b = bars(&[
            10., 9., 8., 7., 6., 5., 4., 3., 2., 1., 2., 4., 8., 12., 16.,
        ]);
        assert_eq!(evaluate(&b, 2, 4).signal, Signal::Hold);
    }
}
