mod advisory;
mod lifecycle;
mod registry;
mod runtime;
mod supervisor_hook;

pub use advisory::{
    assert_advisory_eligible, run_advisory_step, AdvisoryStepInput, AdvisoryStepResult,
};
pub use lifecycle::{pause_agent, resume_agent, retire_agent};
pub use registry::AgentRegistry;
pub use runtime::{monitor_agent_hook_from_env, shared_agent_registry};
pub use supervisor_hook::{MonitorAgentHook, NoopMonitorAgentHook, RegistryMonitorAgentHook};

pub use crate::modules::agents::models::NewAgentSpec;
