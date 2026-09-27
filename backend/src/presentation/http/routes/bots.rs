use axum::{extract::State, http::HeaderMap, Json};

use crate::modules::bots::{BotPromotionRecord, BotRuntimeStatus, PromoteBotRequest};
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
    state
        .bot_catalog_for_config()
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
    State(state): State<ApiState>,
    Json(body): Json<BotRankingRequest>,
) -> Result<Json<BotRankingResponse>, ApiError> {
    state
        .bot_ranking_from_metrics(body.metrics)
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
    headers: HeaderMap,
) -> Result<Json<bots::BotCatalogPersistResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    state
        .persist_bot_catalog()
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
    state
        .bot_catalog_snapshot()
        .await
        .map_err(ApiError::from_bot_error)
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/bots/runtime/status",
    tag = "bots",
    responses(
        (status = 200, description = "Active bot promotion (if any)", body = BotRuntimeStatus),
    )
)]
pub async fn bot_runtime_status(
    State(state): State<ApiState>,
) -> Result<Json<BotRuntimeStatus>, ApiError> {
    Ok(Json(state.bot_runtime_status()))
}

#[utoipa::path(
    post,
    path = "/api/v1/bots/runtime/promote",
    tag = "bots",
    request_body = PromoteBotRequest,
    responses(
        (status = 200, description = "Bot promoted for runtime seam", body = BotPromotionRecord),
        (status = 400, description = "Invalid request", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "Runtime disabled", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn bot_runtime_promote(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<PromoteBotRequest>,
) -> Result<Json<BotPromotionRecord>, ApiError> {
    state.require_http_admin(&headers)?;
    state.promote_bot_http(body).await.map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/bots/runtime/demote",
    tag = "bots",
    responses(
        (status = 204, description = "Promotion cleared"),
        (status = 404, description = "No active promotion", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "Runtime disabled", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn bot_runtime_demote(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<axum::http::StatusCode, ApiError> {
    state.require_http_admin(&headers)?;
    state
        .demote_bot_http()
        .await
        .map_err(ApiError::from_bots_error)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
