use axum::{extract::Query, extract::State, Json};

use crate::modules::http_bridge::config::{self, ConfigSnapshotQuery, ConfigSnapshotResponse};
use crate::presentation::http::{error::ApiError, state::ApiState};

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
    config::load_config_snapshot(query)
        .map_err(ApiError::from_bot_error)
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/config/active",
    tag = "config",
    responses((status = 200, description = "Config loaded at API startup", body = ConfigSnapshotResponse))
)]
pub async fn config_active(State(state): State<ApiState>) -> Json<ConfigSnapshotResponse> {
    Json(config::map_config(state.app_config()))
}
