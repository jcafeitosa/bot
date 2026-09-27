use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::modules::backtest::cli::{execute_backtest, BacktestCli};
use crate::presentation::http::error::ApiError;

#[derive(Debug, Deserialize, ToSchema)]
pub struct BacktestRequest {
    #[schema(example = "src/core/config/bot.toml")]
    pub config: String,
    #[serde(default)]
    pub persist: bool,
}

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
    let cli = BacktestCli {
        config: body.config.into(),
        persist: body.persist,
    };
    execute_backtest(&cli)
        .await
        .map(Json)
        .map_err(ApiError::from_bot_error)
}
