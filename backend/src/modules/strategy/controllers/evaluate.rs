use mantis_ta::{
    indicators::{Indicator, SMA},
    types::Candle,
};

use crate::modules::application_contracts::Signal;
use crate::modules::strategy::models::StrategySnapshot;

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
