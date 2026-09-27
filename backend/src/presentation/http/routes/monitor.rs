use axum::{extract::State, Json};

use crate::modules::http_bridge::monitor::{
    self, MonitorCommandRequest, MonitorCommandResponse, MonitorSnapshotResponse,
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
    let handle = state.monitor().ok_or_else(ApiError::monitor_unavailable)?;
    Ok(Json(monitor::snapshot_response(handle)))
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
    Json(body): Json<MonitorCommandRequest>,
) -> Result<(axum::http::StatusCode, Json<MonitorCommandResponse>), ApiError> {
    let handle = state.monitor().ok_or_else(ApiError::monitor_unavailable)?;
    monitor::send_monitor_command(handle, body).map_err(|error| match error {
        monitor::MonitorCommandHttpError::ChannelFull => ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "monitor command channel full",
        ),
        monitor::MonitorCommandHttpError::ChannelClosed => ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "monitor command channel closed",
        ),
    })?;
    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(MonitorCommandResponse { accepted: true }),
    ))
}
