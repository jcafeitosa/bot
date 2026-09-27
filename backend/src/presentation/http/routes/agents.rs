use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::modules::http_bridge::agents::{
    self, AdvisoryRequest, AdvisoryResponse, AgencyQuery, AgentListResponse, AgentResponse,
    AuditLogResponse, LifecycleResponse, RegisterAgentRequest,
};
use crate::presentation::http::{error::ApiError, state::ApiState};

async fn persist_agent_after_mutation(
    state: &ApiState,
    agency: &str,
    agent_id: &str,
) -> Result<(), ApiError> {
    let postgres = state.database().map(|db| db.as_postgres());
    if postgres.is_none() {
        return Ok(());
    }
    let (definition, event) = state
        .with_agents(|registry| agents::snapshot_for_persist(registry, agency, agent_id))
        .await
        .map_err(ApiError::from_agents_error)?;
    agents::persist_identity_rows(postgres.expect("checked"), &definition, &event)
        .await
        .map_err(ApiError::from_agents_error)
}

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
    state
        .with_agents(|registry| agents::list_agents(registry, &query.agency))
        .await
        .map(Json)
        .map_err(ApiError::from_agents_error)
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
    Json(body): Json<RegisterAgentRequest>,
) -> Result<(StatusCode, Json<AgentResponse>), ApiError> {
    let agency = body.agency.clone();
    let agent_id = body.agent_id.clone();
    let response = state
        .with_agents(|registry| agents::register_agent(registry, body))
        .await
        .map_err(ApiError::from_agents_error)?;
    persist_agent_after_mutation(&state, &agency, &agent_id).await?;
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
    state
        .with_agents(|registry| agents::audit_log(registry, &query.agency))
        .await
        .map(Json)
        .map_err(ApiError::from_agents_error)
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
    state
        .with_agents(|registry| agents::get_agent(registry, &query.agency, &agent_id))
        .await
        .map(Json)
        .map_err(ApiError::from_agents_error)
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
    Path(agent_id): Path<String>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<LifecycleResponse>, ApiError> {
    let agency = query.agency.clone();
    let response = state
        .with_agents(|registry| agents::pause(registry, &agency, &agent_id))
        .await
        .map_err(ApiError::from_agents_error)?;
    persist_agent_after_mutation(&state, &agency, &agent_id).await?;
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
    Path(agent_id): Path<String>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<LifecycleResponse>, ApiError> {
    let agency = query.agency.clone();
    let response = state
        .with_agents(|registry| agents::resume(registry, &agency, &agent_id))
        .await
        .map_err(ApiError::from_agents_error)?;
    persist_agent_after_mutation(&state, &agency, &agent_id).await?;
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
    Path(agent_id): Path<String>,
    Query(query): Query<AgencyQuery>,
) -> Result<Json<LifecycleResponse>, ApiError> {
    let agency = query.agency.clone();
    let response = state
        .with_agents(|registry| agents::retire(registry, &agency, &agent_id))
        .await
        .map_err(ApiError::from_agents_error)?;
    persist_agent_after_mutation(&state, &agency, &agent_id).await?;
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
    Path(agent_id): Path<String>,
    Json(body): Json<AdvisoryRequest>,
) -> Result<Json<AdvisoryResponse>, ApiError> {
    let advisor = state.jev().cloned().ok_or_else(ApiError::jev_unavailable)?;
    let step = state
        .with_agents(|registry| agents::advisory_prepare(registry, &agent_id, body))
        .await
        .map_err(ApiError::from_agents_error)?;
    agents::advisory_finish(&advisor, step)
        .await
        .map_err(ApiError::from_bot_error)
        .map(Json)
}
