#![allow(unused_imports)]
//! Administrative agent identities (`IdentityOnly` foundation) and optional Jev advisory seam.
//!
//! Not `modules/bots` (future trading executors) and not `backtest::BotId` (simulation keys).

pub mod adapters;
pub mod controllers;
pub mod models;

pub use controllers::{
    assert_advisory_eligible, monitor_agent_hook_from_env, pause_agent, resume_agent, retire_agent,
    run_advisory_step, AdvisoryStepInput, AdvisoryStepResult, AgentRegistry, MonitorAgentHook,
    NewAgentSpec, NoopMonitorAgentHook, RegistryMonitorAgentHook,
};
pub use models::{
    validate_hierarchy, AgencyId, AgentCapabilities, AgentDefinition, AgentId, AgentLifecycleState,
    AgentRole, AgentsError, IdentityAuditEvent, IdentityEventKind, OwnerId, SupervisorRef,
};

#[cfg(test)]
mod tests;
