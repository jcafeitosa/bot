#![allow(dead_code)]
mod definition;
mod hierarchy;
mod identity;
mod product_owner;

pub use definition::{
    AgentCapabilities, AgentDefinition, AgentLifecycleState, IdentityAuditEvent, IdentityEventKind,
};
pub use hierarchy::{validate_hierarchy, AgentRole, SupervisorRef};
pub use identity::{AgencyId, AgentId, AgentsError, OwnerId};
pub use product_owner::VerifiedProductOwner;

mod spec;

pub use spec::NewAgentSpec;
