use mantis_ta::{
    indicators::{Indicator, EMA, SMA},
    types::Candle,
};

use crate::modules::application_contracts::Signal;
use crate::modules::bots::models::MonitorEvaluatorKind;
use crate::modules::strategy::models::StrategySnapshot;

pub fn evaluate_for_kind(
    kind: MonitorEvaluatorKind,
    candles: &[Candle],
    fast: usize,
    slow: usize,
) -> StrategySnapshot {
    match kind {
        MonitorEvaluatorKind::SmaCross => evaluate(candles, fast, slow),
        MonitorEvaluatorKind::EmaCross => evaluate_ema(candles, fast, slow),
    }
}

pub fn evaluate(candles: &[Candle], fast: usize, slow: usize) -> StrategySnapshot {
    crossover_evaluate(
        candles,
        fast,
        slow,
        |bars| SMA::new(fast).calculate(bars),
        |bars| SMA::new(slow).calculate(bars),
    )
}

pub fn evaluate_ema(candles: &[Candle], fast: usize, slow: usize) -> StrategySnapshot {
    crossover_evaluate(
        candles,
        fast,
        slow,
        |bars| EMA::new(fast).calculate(bars),
        |bars| EMA::new(slow).calculate(bars),
    )
}

fn crossover_evaluate(
    candles: &[Candle],
    fast: usize,
    slow: usize,
    fast_series: impl FnOnce(&[Candle]) -> Vec<Option<f64>>,
    slow_series: impl FnOnce(&[Candle]) -> Vec<Option<f64>>,
) -> StrategySnapshot {
    if candles.is_empty() || fast == 0 || fast >= slow {
        return StrategySnapshot {
            signal: Signal::Warmup,
            close: candles.last().map(|c| c.close).unwrap_or_default(),
            fast_sma: None,
            slow_sma: None,
            candle_timestamp_ms: candles.last().map(|c| c.timestamp).unwrap_or_default(),
        };
    }
    let fast_values = fast_series(candles);
    let slow_values = slow_series(candles);
    let last = candles.last().expect("caller must check nonempty candles");
    let n = candles.len();
    let fast_now = fast_values[n - 1];
    let slow_now = slow_values[n - 1];
    let signal = crossover_signal(fast_now, slow_now, n, &fast_values, &slow_values);
    StrategySnapshot {
        signal,
        close: last.close,
        fast_sma: fast_now,
        slow_sma: slow_now,
        candle_timestamp_ms: last.timestamp,
    }
}

fn crossover_signal(
    fast_now: Option<f64>,
    slow_now: Option<f64>,
    n: usize,
    fast_values: &[Option<f64>],
    slow_values: &[Option<f64>],
) -> Signal {
    match (
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
    }
}
