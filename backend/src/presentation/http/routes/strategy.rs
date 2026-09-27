use axum::{extract::Query, Json};

use crate::modules::http_bridge::strategy::{
    self, EvaluateSmaRequest, EvaluateSmaResponse, PeriodsQuery, StrategyPeriodsResponse,
};

#[utoipa::path(
    get,
    path = "/api/v1/strategy/periods",
    tag = "strategy",
    params(PeriodsQuery),
    responses((status = 200, description = "SMA preset for operation mode", body = StrategyPeriodsResponse))
)]
pub async fn sma_periods(Query(query): Query<PeriodsQuery>) -> Json<StrategyPeriodsResponse> {
    Json(strategy::sma_periods(query))
}

#[utoipa::path(
    post,
    path = "/api/v1/strategy/evaluate-sma",
    tag = "strategy",
    request_body = EvaluateSmaRequest,
    responses((status = 200, description = "SMA crossover evaluation", body = EvaluateSmaResponse))
)]
pub async fn evaluate_sma(Json(body): Json<EvaluateSmaRequest>) -> Json<EvaluateSmaResponse> {
    Json(strategy::evaluate_sma(body))
}
