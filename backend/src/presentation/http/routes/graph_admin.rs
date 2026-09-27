use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};

use crate::core::database::ProjectedAgentList;
use crate::presentation::http::{error::ApiError, state::ApiState};

#[derive(Debug, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct GraphAgentsQuery {
    /// Maximum rows to return (1–500).
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/graph/agents",
    tag = "admin",
    params(GraphAgentsQuery),
    responses(
        (status = 200, description = "Advisory read-only agents projected from Neo4j (PG remains SoT)", body = ProjectedAgentList),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL or Neo4j not wired", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn list_graph_agents(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(query): Query<GraphAgentsQuery>,
) -> Result<Json<ProjectedAgentList>, ApiError> {
    state.require_http_admin(&headers)?;
    let limit = query.limit.unwrap_or(32).clamp(1, 500);
    state.list_graph_agents_advisory(limit).await.map(Json)
}
