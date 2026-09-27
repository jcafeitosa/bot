use axum::Json;

use crate::modules::http_bridge::backtest::{self, BacktestRequest};
use crate::presentation::http::error::ApiError;

#[utoipa::path(
    post,
    path = "/api/v1/backtest/sma-crossover",
    tag = "backtest",
    request_body = BacktestRequest,
    responses(
        (status = 200, description = "Backtest summary JSON", body = serde_json::Value),
        (status = 400, description = "Invalid config or parameters", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn run_sma_backtest(
    Json(body): Json<BacktestRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    backtest::run_sma_crossover(body)
        .await
        .map(Json)
        .map_err(ApiError::from_bot_error)
}
