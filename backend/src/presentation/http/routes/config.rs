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

#[utoipa::path(
    get,
    path = "/api/v1/config/active",
    tag = "config",
    responses((status = 200, description = "Config loaded at API startup", body = ConfigSnapshotResponse))
)]
pub async fn config_active(
    axum::extract::State(state): axum::extract::State<crate::presentation::http::state::ApiState>,
) -> Json<ConfigSnapshotResponse> {
    Json(config::map_config(state.app_config()))
}
