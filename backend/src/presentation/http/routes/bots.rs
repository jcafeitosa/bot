use axum::{extract::State, Json};

use crate::modules::http_bridge::bots::{
    self, BotCatalogResponse, BotRankingRequest, BotRankingResponse,
};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[utoipa::path(
    get,
    path = "/api/v1/bots/catalog",
    tag = "bots",
    responses(
        (status = 200, description = "Bot instances for active config operation", body = BotCatalogResponse),
        (status = 400, description = "Invalid config", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn bot_catalog(
    State(state): State<ApiState>,
) -> Result<Json<BotCatalogResponse>, ApiError> {
    bots::catalog_for_config(state.app_config())
        .map_err(ApiError::from_bot_error)
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/bots/ranking",
    tag = "bots",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Full bot ranking for homogeneous metrics batch", body = serde_json::Value),
        (status = 400, description = "Invalid metrics or incompatible scope", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn bot_ranking(
    Json(body): Json<BotRankingRequest>,
) -> Result<Json<BotRankingResponse>, ApiError> {
    bots::ranking_from_metrics(body.metrics)
        .map_err(ApiError::from_bot_error)
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/bots/catalog/persist",
    tag = "bots",
    responses(
        (status = 200, description = "Build catalog and persist via store seam", body = bots::BotCatalogPersistResponse),
        (status = 400, description = "Invalid config", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn bot_catalog_persist(
    State(state): State<ApiState>,
) -> Result<Json<bots::BotCatalogPersistResponse>, ApiError> {
    let config = state.app_config().clone();
    let mut guard = state.bot_catalog().lock().await;
    bots::persist_catalog_for_config(&config, &mut *guard)
        .await
        .map_err(ApiError::from_bot_error)
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/bots/catalog/snapshot",
    tag = "bots",
    responses(
        (status = 200, description = "Last catalog snapshot from POST /bots/catalog/persist", body = BotCatalogResponse),
        (status = 400, description = "Store read error", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn bot_catalog_snapshot(
    State(state): State<ApiState>,
) -> Result<Json<BotCatalogResponse>, ApiError> {
    let guard = state.bot_catalog().lock().await;
    bots::catalog_from_store(&*guard)
        .await
        .map_err(ApiError::from_bot_error)
        .map(Json)
}
