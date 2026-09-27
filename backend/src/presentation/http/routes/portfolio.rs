use axum::{extract::Query, Json};

use crate::modules::http_bridge::portfolio::{self, PaperSnapshotQuery, PaperSnapshotResponse};
use crate::presentation::http::error::ApiError;

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
    Query(query): Query<PaperSnapshotQuery>,
) -> Result<Json<PaperSnapshotResponse>, ApiError> {
    portfolio::paper_wallet_snapshot(query)
        .map_err(ApiError::from_bot_error)
        .map(Json)
}
