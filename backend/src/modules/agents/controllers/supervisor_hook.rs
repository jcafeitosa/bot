use crate::modules::agents::models::AgentId;

/// Future integration point for `modules::monitor::controllers::supervisor`.
pub trait MonitorAgentHook {
    fn on_evaluation_cycle(&self, agent_ids: &[AgentId]);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopMonitorAgentHook;

impl MonitorAgentHook for NoopMonitorAgentHook {
    fn on_evaluation_cycle(&self, _agent_ids: &[AgentId]) {}
}
