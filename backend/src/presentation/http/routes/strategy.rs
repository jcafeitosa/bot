use axum::{extract::Query, Json};

use crate::modules::http_bridge::strategy::{self, PeriodsQuery, StrategyPeriodsResponse};

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
