use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::modules::monitor::MonitorCommand;
use crate::presentation::http::{error::ApiError, state::ApiState};

#[derive(Debug, Serialize, ToSchema)]
pub struct MonitorSnapshotResponse {
    pub revision: u64,
    pub run_state: String,
    pub symbol: String,
    pub operation_label: String,
    pub risk_profile_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MonitorCommandName {
    Pause,
    Resume,
    Refresh,
    Shutdown,
}

impl From<MonitorCommandName> for MonitorCommand {
    fn from(value: MonitorCommandName) -> Self {
        match value {
            MonitorCommandName::Pause => MonitorCommand::Pause,
            MonitorCommandName::Resume => MonitorCommand::Resume,
            MonitorCommandName::Refresh => MonitorCommand::Refresh,
            MonitorCommandName::Shutdown => MonitorCommand::Shutdown,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MonitorCommandRequest {
    pub command: MonitorCommandName,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MonitorCommandResponse {
    pub accepted: bool,
}

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
    let snapshot = handle.latest_snapshot().borrow().clone();
    Ok(Json(MonitorSnapshotResponse {
        revision: snapshot.revision,
        run_state: format!("{:?}", snapshot.run_state),
        symbol: snapshot.symbol,
        operation_label: snapshot.operation_label,
        risk_profile_label: snapshot.risk_profile_label,
        signal: snapshot
            .market
            .and_then(|market| market.signal.map(|value| format!("{:?}", value))),
        last_error: snapshot.last_error,
    }))
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
    handle
        .send(body.command.into())
        .map_err(|error| match error {
            crate::modules::monitor::MonitorSendError::Full => ApiError::new(
                axum::http::StatusCode::CONFLICT,
                "monitor command channel full",
            ),
            crate::modules::monitor::MonitorSendError::Closed => ApiError::new(
                axum::http::StatusCode::CONFLICT,
                "monitor command channel closed",
            ),
        })?;
    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(MonitorCommandResponse { accepted: true }),
    ))
}
