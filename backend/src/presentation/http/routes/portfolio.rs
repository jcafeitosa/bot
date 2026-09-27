use axum::{
    extract::{Query, State},
    Json,
};

use crate::modules::http_bridge::portfolio::{PaperSnapshotQuery, PaperSnapshotResponse};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[utoipa::path(
    get,
    path = "/api/v1/portfolio/paper-snapshot",
    tag = "portfolio",
    params(PaperSnapshotQuery),
    responses(
        (status = 200, description = "Paper wallet baseline", body = PaperSnapshotResponse),
        (status = 400, description = "Invalid asset", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn paper_wallet(
    State(state): State<ApiState>,
    Query(query): Query<PaperSnapshotQuery>,
) -> Result<Json<PaperSnapshotResponse>, ApiError> {
    state
        .paper_wallet_snapshot(query)
        .map_err(ApiError::from_bot_error)
        .map(Json)
}
