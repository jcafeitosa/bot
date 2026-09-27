use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::modules::monitor::{MonitorCommand, MonitorHandle, MonitorSendError, MonitorSnapshot};

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

pub fn snapshot_response(handle: &MonitorHandle) -> MonitorSnapshotResponse {
    let snapshot = handle.latest_snapshot().borrow().clone();
    snapshot_from_domain(snapshot)
}

pub fn snapshot_from_domain(snapshot: MonitorSnapshot) -> MonitorSnapshotResponse {
    MonitorSnapshotResponse {
        revision: snapshot.revision,
        run_state: format!("{:?}", snapshot.run_state),
        symbol: snapshot.symbol,
        operation_label: snapshot.operation_label,
        risk_profile_label: snapshot.risk_profile_label,
        signal: snapshot
            .market
            .and_then(|market| market.signal.map(|value| format!("{:?}", value))),
        last_error: snapshot.last_error,
    }
}

pub fn send_monitor_command(
    handle: &MonitorHandle,
    body: MonitorCommandRequest,
) -> Result<(), MonitorSendError> {
    handle.send(body.command.into())
}
