use serde::Serialize;

use crate::modules::application_contracts::Signal;

#[derive(Debug, Clone, Serialize)]
pub struct StrategySnapshot {
    pub signal: Signal,
    pub close: f64,
    pub fast_sma: Option<f64>,
    pub slow_sma: Option<f64>,
    pub candle_timestamp_ms: i64,
}
