use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct JevReviewInput {
    pub signal_label: &'static str,
    pub close: f64,
    pub fast_sma: Option<f64>,
    pub slow_sma: Option<f64>,
    pub candle_timestamp_ms: i64,
}

#[derive(Debug, Deserialize)]
pub struct JevResponse {
    pub model: Option<String>,
    pub answers: std::collections::HashMap<String, Value>,
}
