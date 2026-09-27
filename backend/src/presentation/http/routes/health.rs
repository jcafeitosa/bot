use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::core::database::{
    fetch_graph_projection_outbox_stats, graph_projection_outbox_degraded,
    GraphProjectionOutboxStats,
};
use crate::core::health::{liveness, readiness_databases, ProbeStatus};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[derive(Debug, Serialize, ToSchema)]
pub struct GraphProjectionOutboxHealth {
    pub pending: u64,
    pub retry: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_pending_age_secs: Option<u64>,
    pub degraded: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_projection_outbox: Option<GraphProjectionOutboxHealth>,
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
pub async fn healthz(State(state): State<ApiState>) -> Json<HealthResponse> {
    let _ = liveness();
    let outbox = graph_projection_outbox_health(state.databases()).await;
    let status = if outbox.as_ref().is_some_and(|o| o.degraded) {
        "degraded"
    } else {
        "ok"
    };
    Json(HealthResponse {
        status,
        graph_projection_outbox: outbox,
    })
}

async fn graph_projection_outbox_health(
    databases: &crate::core::database::AppDatabases,
) -> Option<GraphProjectionOutboxHealth> {
    databases.neo4j()?;
    let pool = databases.postgres_handle()?.pool();
    let stats = fetch_graph_projection_outbox_stats(pool).await.ok()?;
    Some(stats_to_health(&stats))
}

fn stats_to_health(stats: &GraphProjectionOutboxStats) -> GraphProjectionOutboxHealth {
    GraphProjectionOutboxHealth {
        pending: stats.pending,
        retry: stats.retry,
        oldest_pending_age_secs: stats.oldest_pending_age_secs,
        degraded: graph_projection_outbox_degraded(stats),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_to_health_marks_degraded_on_backlog() {
        let health = stats_to_health(&GraphProjectionOutboxStats {
            pending: 2,
            retry: 0,
            oldest_pending_age_secs: Some(10),
        });
        assert!(health.degraded);
        assert_eq!(health.pending, 2);
    }
}
