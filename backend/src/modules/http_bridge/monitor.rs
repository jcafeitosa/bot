use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::modules::bots::BotRuntimeStatus;
use crate::modules::monitor::{MonitorCommand, MonitorHandle, MonitorSendError, MonitorSnapshot};

#[derive(Debug, Serialize, ToSchema)]
pub struct MonitorSnapshotResponse {
    pub revision: u64,
    pub run_state: String,
    pub symbol: String,
    pub operation_label: String,
    pub risk_profile_label: String,
    /// Archive session health from the monitor persistence seam (`healthy` / `degraded` / `gap` / `unavailable`).
    pub persistence_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// Active bot promotion from `BotRuntimePort` (HTTP API process); supervisor sets `BotSignal::bot_id` when promotion matches market.
    pub bot_runtime_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promoted_bot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promoted_by: Option<String>,
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
        persistence_status: snapshot.persistence_status.as_api_str().into(),
        signal: snapshot
            .market
            .and_then(|market| market.signal.map(|value| format!("{:?}", value))),
        last_error: snapshot.last_error,
        bot_runtime_enabled: snapshot.bot_runtime_enabled,
        promoted_bot_id: snapshot.promoted_bot_id.clone(),
        promoted_by: snapshot.promoted_by.clone(),
    }
}

pub fn attach_bot_runtime_status(
    response: MonitorSnapshotResponse,
    runtime: BotRuntimeStatus,
) -> MonitorSnapshotResponse {
    let promoted_bot_id = runtime.active.as_ref().map(|r| r.bot_id.clone());
    let promoted_by = runtime.active.as_ref().map(|r| r.promoted_by.clone());
    MonitorSnapshotResponse {
        bot_runtime_enabled: runtime.runtime_enabled,
        promoted_bot_id,
        promoted_by,
        ..response
    }
}

pub fn send_monitor_command(
    handle: &MonitorHandle,
    body: MonitorCommandRequest,
) -> Result<(), MonitorCommandHttpError> {
    handle
        .send(body.command.into())
        .map_err(MonitorCommandHttpError::from)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorCommandHttpError {
    ChannelFull,
    ChannelClosed,
}

impl From<MonitorSendError> for MonitorCommandHttpError {
    fn from(value: MonitorSendError) -> Self {
        match value {
            MonitorSendError::Full => Self::ChannelFull,
            MonitorSendError::Closed => Self::ChannelClosed,
        }
    }
}

#[cfg(test)]
mod monitor_bridge_tests {
    use super::*;
    use crate::modules::bots::{BotPromotionRecord, BotPromotionState, BotRuntimeStatus};
    use crate::modules::monitor::MonitorSnapshot;

    #[test]
    fn snapshot_from_domain_exposes_persistence_status_for_rest() {
        use crate::modules::monitor::PersistenceStatus;
        let mut snapshot = MonitorSnapshot::initial();
        snapshot.revision = 4;
        snapshot.persistence_status = PersistenceStatus::Gap;
        let response = snapshot_from_domain(snapshot);
        assert_eq!(response.persistence_status, "gap");
    }

    #[test]
    fn attach_bot_runtime_status_enriches_snapshot_fields() {
        let base = snapshot_from_domain(MonitorSnapshot::initial());
        assert!(!base.bot_runtime_enabled);
        assert!(base.promoted_bot_id.is_none());

        let enriched = attach_bot_runtime_status(
            base,
            BotRuntimeStatus {
                runtime_enabled: true,
                active: Some(BotPromotionRecord {
                    bot_id: "sma-cross@1:5m:BTC/USDT".into(),
                    promoted_by: "owner-1".into(),
                    promoted_at_unix_ms: 42,
                    state: BotPromotionState::Active,
                }),
            },
        );
        assert!(enriched.bot_runtime_enabled);
        assert_eq!(
            enriched.promoted_bot_id.as_deref(),
            Some("sma-cross@1:5m:BTC/USDT")
        );
        assert_eq!(enriched.promoted_by.as_deref(), Some("owner-1"));
    }

    #[test]
    fn attach_bot_runtime_status_preserves_registry_v2_bot_id() {
        let enriched = attach_bot_runtime_status(
            snapshot_from_domain(MonitorSnapshot::initial()),
            BotRuntimeStatus {
                runtime_enabled: true,
                active: Some(BotPromotionRecord {
                    bot_id: "sma-cross@2:15m:BTCUSDT".into(),
                    promoted_by: "owner-1".into(),
                    promoted_at_unix_ms: 1,
                    state: BotPromotionState::Active,
                }),
            },
        );
        assert_eq!(
            enriched.promoted_bot_id.as_deref(),
            Some("sma-cross@2:15m:BTCUSDT")
        );
    }
}
