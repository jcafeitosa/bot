use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::core::health::{liveness, readiness_databases, ProbeStatus};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadyResponse {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neo4j: Option<&'static str>,
}

#[utoipa::path(
    get,
    path = "/healthz",
    tag = "system",
    responses((status = 200, description = "Process is alive", body = HealthResponse))
)]
pub async fn healthz() -> Json<HealthResponse> {
    let _ = liveness();
    Json(HealthResponse { status: "ok" })
}

#[utoipa::path(
    get,
    path = "/readyz",
    tag = "system",
    responses(
        (status = 200, description = "Dependencies reachable", body = ReadyResponse),
        (status = 503, description = "Dependency check failed", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn readyz(State(state): State<ApiState>) -> Result<Json<ReadyResponse>, ApiError> {
    let report = readiness_databases(state.databases()).await;
    if !report.ready {
        let detail = report
            .components
            .iter()
            .find(|component| component.status == ProbeStatus::Down)
            .and_then(|component| component.detail.clone())
            .unwrap_or_else(|| "dependency check failed".into());
        return Err(ApiError::new(StatusCode::SERVICE_UNAVAILABLE, detail));
    }

    let database = if state.database().is_some() {
        Some("ok")
    } else {
        None
    };
    let neo4j = if state.databases().neo4j().is_some() {
        Some("ok")
    } else {
        None
    };
    Ok(Json(ReadyResponse {
        status: "ready",
        database,
        neo4j,
    }))
}
