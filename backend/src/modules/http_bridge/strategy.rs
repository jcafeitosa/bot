use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::modules::application_contracts::{signal_label, Signal};
use crate::modules::config_api::OperationMode;
use crate::modules::market::models::Candle;
use crate::modules::strategy::evaluate;
use crate::modules::strategy::periods_for_mode;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct PeriodsQuery {
    pub operation: OperationMode,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StrategyPeriodsResponse {
    pub operation: OperationMode,
    pub fast_period: usize,
    pub slow_period: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EvaluateSmaRequest {
    pub fast_period: usize,
    pub slow_period: usize,
    pub candles: Vec<Candle>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EvaluateSmaResponse {
    pub signal: Signal,
    pub signal_label: String,
    pub close: f64,
    pub fast_sma: Option<f64>,
    pub slow_sma: Option<f64>,
    pub candle_timestamp_ms: i64,
}

pub fn sma_periods(query: PeriodsQuery) -> StrategyPeriodsResponse {
    let (fast_period, slow_period) = periods_for_mode(query.operation);
    StrategyPeriodsResponse {
        operation: query.operation,
        fast_period,
        slow_period,
    }
}

pub fn evaluate_sma(body: EvaluateSmaRequest) -> EvaluateSmaResponse {
    let mantis: Vec<_> = body.candles.iter().map(|c| c.to_mantis()).collect();
    let snapshot = evaluate(&mantis, body.fast_period, body.slow_period);
    EvaluateSmaResponse {
        signal_label: signal_label(snapshot.signal).to_owned(),
        signal: snapshot.signal,
        close: snapshot.close,
        fast_sma: snapshot.fast_sma,
        slow_sma: snapshot.slow_sma,
        candle_timestamp_ms: snapshot.candle_timestamp_ms,
    }
}
