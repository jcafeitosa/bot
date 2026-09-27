use axum::{extract::State, http::HeaderMap, Json};

use crate::modules::http_bridge::monitor::{
    MonitorCommandRequest, MonitorCommandResponse, MonitorSnapshotResponse,
};
use crate::presentation::http::{error::ApiError, state::ApiState};

#[utoipa::path(
    get,
    path = "/api/v1/monitor/snapshot",
    tag = "monitor",
    responses(
        (status = 200, description = "Latest monitor snapshot", body = MonitorSnapshotResponse),
        (status = 503, description = "Monitor not attached", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn snapshot(
    State(state): State<ApiState>,
) -> Result<Json<MonitorSnapshotResponse>, ApiError> {
    Ok(Json(state.monitor_snapshot()?))
}

#[utoipa::path(
    post,
    path = "/api/v1/monitor/commands",
    tag = "monitor",
    request_body = MonitorCommandRequest,
    responses(
        (status = 202, description = "Command accepted", body = MonitorCommandResponse),
        (status = 503, description = "Monitor not attached", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn post_command(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<MonitorCommandRequest>,
) -> Result<(axum::http::StatusCode, Json<MonitorCommandResponse>), ApiError> {
    state.require_http_admin(&headers)?;
    state.accept_monitor_command(body)?;
    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(MonitorCommandResponse { accepted: true }),
    ))
}
