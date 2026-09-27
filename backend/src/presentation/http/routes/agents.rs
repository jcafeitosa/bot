use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::modules::http_bridge::agents::{
    AdvisoryRequest, AdvisoryResponse, AgencyQuery, AgentListResponse, AgentResponse,
    AuditLogResponse, LifecycleResponse, RegisterAgentRequest,
};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[utoipa::path(
    get,
    path = "/api/v1/agents",
    tag = "agents",
    params(AgencyQuery),
    responses((status = 200, description = "Agents in agency", body = AgentListResponse))
)]
pub async fn list_agents(
    State(state): State<ApiState>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<AgentListResponse>, ApiError> {
    state.require_bound_agency(&query.agency)?;
    state.list_agents_in_agency(&query.agency).await.map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/agents",
    tag = "agents",
    request_body = RegisterAgentRequest,
    responses(
        (status = 201, description = "Registered", body = AgentResponse),
        (status = 400, description = "Invalid spec", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn register_agent(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<RegisterAgentRequest>,
) -> Result<(StatusCode, Json<AgentResponse>), ApiError> {
    state.require_http_admin(&headers)?;
    state.require_register_owner_id(&body.owner_id)?;
    state.require_bound_agency(&body.agency)?;
    let response = state.register_agent_and_persist(body).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/audit",
    tag = "agents",
    params(AgencyQuery),
    responses((status = 200, description = "Identity audit trail", body = AuditLogResponse))
)]
pub async fn audit_log(
    State(state): State<ApiState>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<AuditLogResponse>, ApiError> {
    state.require_bound_agency(&query.agency)?;
    state.agents_audit_log(&query.agency).await.map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/{agent_id}",
    tag = "agents",
    params(AgencyQuery),
    responses(
        (status = 200, description = "Agent", body = AgentResponse),
        (status = 404, description = "Not found", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn get_agent(
    State(state): State<ApiState>,
    Path(agent_id): Path<String>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<AgentResponse>, ApiError> {
    state.require_bound_agency(&query.agency)?;
    state
        .get_agent_in_agency(&query.agency, &agent_id)
        .await
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent_id}/pause",
    tag = "agents",
    params(AgencyQuery),
    responses((status = 200, description = "Paused", body = LifecycleResponse))
)]
pub async fn pause_agent(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<LifecycleResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    state.require_bound_agency(&query.agency)?;
    let response = state
        .pause_agent_and_persist(&query.agency, &agent_id)
        .await?;
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent_id}/resume",
    tag = "agents",
    params(AgencyQuery),
    responses((status = 200, description = "Resumed", body = LifecycleResponse))
)]
pub async fn resume_agent(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<LifecycleResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    state.require_bound_agency(&query.agency)?;
    let response = state
        .resume_agent_and_persist(&query.agency, &agent_id)
        .await?;
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent_id}/retire",
    tag = "agents",
    params(AgencyQuery),
    responses((status = 200, description = "Retired", body = LifecycleResponse))
)]
pub async fn retire_agent(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<LifecycleResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    state.require_bound_agency(&query.agency)?;
    let response = state
        .retire_agent_and_persist(&query.agency, &agent_id)
        .await?;
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent_id}/advisory",
    tag = "agents",
    request_body = AdvisoryRequest,
    responses(
        (status = 200, description = "Advisory lines", body = AdvisoryResponse),
        (status = 503, description = "Jev unavailable", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn run_advisory(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(body): Json<AdvisoryRequest>,
) -> Result<Json<AdvisoryResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    state.require_bound_agency(&body.agency)?;
    state.run_agent_advisory(&agent_id, body).await.map(Json)
}
