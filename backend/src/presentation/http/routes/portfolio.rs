use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::modules::portfolio::{paper_snapshot, Asset};
use crate::presentation::http::error::ApiError;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct PaperSnapshotQuery {
    #[param(example = "usdt")]
    pub quote: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaperSnapshotResponse {
    pub quote: String,
    pub as_of_ms: i64,
    pub available: String,
    pub locked: String,
}

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
    let asset = Asset::new(query.quote)
        .map_err(|error| ApiError::new(axum::http::StatusCode::BAD_REQUEST, error.to_string()))?;
    let snapshot = paper_snapshot(&asset);
    let balance = snapshot.balances.first().ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "missing balance",
        )
    })?;
    Ok(Json(PaperSnapshotResponse {
        quote: asset.as_str().to_owned(),
        as_of_ms: snapshot.as_of_ms,
        available: balance.available.to_string(),
        locked: balance.locked.to_string(),
    }))
}
