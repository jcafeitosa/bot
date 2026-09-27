use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};

use crate::core::database::{
    ProjectedAgentList, ProjectedBotsForAgent, ProjectedCodeImpactForModule,
    ProjectedSupervisionChain,
};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[derive(Debug, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct GraphAgentsQuery {
    /// Maximum rows to return (1–500).
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct GraphSupervisionChainQuery {
    pub agency_id: String,
    pub agent_id: String,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct GraphBotsForAgentQuery {
    pub agency_id: String,
    pub agent_id: String,
    /// Maximum rows to return (1–500).
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct GraphCodeImpactQuery {
    /// Module path (e.g. `modules/orders` or `backend/src/modules/orders`).
    pub module_path: String,
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

#[utoipa::path(
    get,
    path = "/api/v1/admin/graph/supervision-chain",
    tag = "admin",
    params(GraphSupervisionChainQuery),
    responses(
        (status = 200, description = "Advisory supervision chain from Neo4j (PG remains SoT)", body = ProjectedSupervisionChain),
        (status = 400, description = "Invalid agency_id or agent_id", body = crate::presentation::http::error::ApiErrorBody),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL or Neo4j not wired", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn graph_supervision_chain(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(query): Query<GraphSupervisionChainQuery>,
) -> Result<Json<ProjectedSupervisionChain>, ApiError> {
    state.require_http_admin(&headers)?;
    state
        .graph_supervision_chain_advisory(&query.agency_id, &query.agent_id)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/graph/bots-for-agent",
    tag = "admin",
    params(GraphBotsForAgentQuery),
    responses(
        (status = 200, description = "Advisory bots promoted by agent from Neo4j (PG remains SoT)", body = ProjectedBotsForAgent),
        (status = 400, description = "Invalid agency_id or agent_id", body = crate::presentation::http::error::ApiErrorBody),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL or Neo4j not wired", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn graph_bots_for_agent(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(query): Query<GraphBotsForAgentQuery>,
) -> Result<Json<ProjectedBotsForAgent>, ApiError> {
    state.require_http_admin(&headers)?;
    let limit = query.limit.unwrap_or(32).clamp(1, 500);
    state
        .graph_bots_for_agent_advisory(&query.agency_id, &query.agent_id, limit)
        .await
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/graph/code-impact",
    tag = "admin",
    params(GraphCodeImpactQuery),
    responses(
        (status = 200, description = "Advisory code impact entities for a module from Neo4j (PG remains SoT)", body = ProjectedCodeImpactForModule),
        (status = 400, description = "Invalid module_path", body = crate::presentation::http::error::ApiErrorBody),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL or Neo4j not wired", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn graph_code_impact(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(query): Query<GraphCodeImpactQuery>,
) -> Result<Json<ProjectedCodeImpactForModule>, ApiError> {
    state.require_http_admin(&headers)?;
    let limit = query.limit.unwrap_or(32).clamp(1, 500);
    state
        .graph_code_impact_for_module_advisory(&query.module_path, limit)
        .await
        .map(Json)
}
