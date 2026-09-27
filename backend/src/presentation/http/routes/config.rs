use axum::{extract::Query, Json};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::modules::http_bridge::config::{self, ConfigSnapshotResponse};
use crate::presentation::http::error::ApiError;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ConfigSnapshotQuery {
    #[param(example = "src/core/config/bot.toml")]
    pub config: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/config/snapshot",
    tag = "config",
    params(ConfigSnapshotQuery),
    responses(
        (status = 200, description = "Non-secret config summary", body = ConfigSnapshotResponse),
        (status = 400, description = "Invalid config path", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn config_snapshot(
    Query(query): Query<ConfigSnapshotQuery>,
) -> Result<Json<ConfigSnapshotResponse>, ApiError> {
    config::load_config_snapshot(config::ConfigSnapshotQuery {
        config: query.config,
    })
    .map_err(ApiError::from_bot_error)
    .map(Json)
}
