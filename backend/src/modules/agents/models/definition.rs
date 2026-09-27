use super::hierarchy::{AgentRole, SupervisorRef};
use super::identity::{AgencyId, AgentId, OwnerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentLifecycleState {
    Active,
    Paused,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentCapabilities {
    /// Explicit opt-in for Jev advisory; IdentityOnly agents keep this false.
    pub consult_jev: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDefinition {
    pub id: AgentId,
    pub agency: AgencyId,
    pub owner: OwnerId,
    pub display_name: String,
    pub role: AgentRole,
    pub supervisor: SupervisorRef,
    pub state: AgentLifecycleState,
    pub capabilities: AgentCapabilities,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityEventKind {
    Registered,
    Paused,
    Resumed,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityAuditEvent {
    pub agent_id: AgentId,
    pub agency: AgencyId,
    pub kind: IdentityEventKind,
    pub at_ms: i64,
}
