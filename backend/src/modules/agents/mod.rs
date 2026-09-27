#![allow(unused_imports)]
//! Administrative agent identities (`IdentityOnly` foundation) and optional Jev advisory seam.

pub mod adapters;
pub mod controllers;
pub mod models;

pub use controllers::{
    assert_advisory_eligible, pause_agent, resume_agent, retire_agent, run_advisory_step,
    AdvisoryStepInput, AdvisoryStepResult, AgentRegistry, MonitorAgentHook, NewAgentSpec,
    NoopMonitorAgentHook,
};
pub use models::{
    validate_hierarchy, AgencyId, AgentCapabilities, AgentDefinition, AgentId, AgentLifecycleState,
    AgentRole, AgentsError, IdentityAuditEvent, IdentityEventKind, OwnerId, SupervisorRef,
};

#[cfg(test)]
mod tests;
