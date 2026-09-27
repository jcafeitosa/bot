use super::definition::AgentCapabilities;
use super::hierarchy::{AgentRole, SupervisorRef};
use super::identity::{AgencyId, AgentId, OwnerId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAgentSpec {
    pub id: AgentId,
    pub agency: AgencyId,
    pub owner: OwnerId,
    pub display_name: String,
    pub role: AgentRole,
    pub supervisor: SupervisorRef,
    pub capabilities: AgentCapabilities,
}
