#![allow(dead_code)] // Registry hook seam; monitor wiring pending.
use std::sync::{Arc, Mutex};

use crate::modules::agents::controllers::registry::AgentRegistry;
use crate::modules::agents::models::{AgencyId, AgentId, AgentLifecycleState};

/// Integration point for `modules::monitor::controllers::supervisor`.
pub trait MonitorAgentHook: Send + Sync {
    fn evaluation_agents(&self) -> Vec<AgentId> {
        Vec::new()
    }

    fn on_evaluation_cycle(&self, agent_ids: &[AgentId]);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopMonitorAgentHook;

impl MonitorAgentHook for NoopMonitorAgentHook {
    fn on_evaluation_cycle(&self, _agent_ids: &[AgentId]) {}
}

/// Lists active agents with `consult_jev` in the configured agency when the monitor evaluates a bar.
#[derive(Clone)]
#[allow(dead_code)] // Wired when monitor passes MonitorAgentHook from agent registry.
pub struct RegistryMonitorAgentHook {
    registry: Arc<Mutex<AgentRegistry>>,
    agency: AgencyId,
}

impl RegistryMonitorAgentHook {
    pub fn new(registry: Arc<Mutex<AgentRegistry>>, agency: AgencyId) -> Self {
        Self { registry, agency }
    }

    fn consultant_ids(&self) -> Vec<AgentId> {
        let guard = self.registry.lock().expect("agent registry lock poisoned");
        guard
            .list_agency(&self.agency)
            .into_iter()
            .filter(|agent| {
                agent.state == AgentLifecycleState::Active && agent.capabilities.consult_jev
            })
            .map(|agent| agent.id.clone())
            .collect()
    }
}

impl MonitorAgentHook for RegistryMonitorAgentHook {
    fn evaluation_agents(&self) -> Vec<AgentId> {
        self.consultant_ids()
    }

    fn on_evaluation_cycle(&self, _agent_ids: &[AgentId]) {
        let consultants = self.consultant_ids();
        if consultants.is_empty() {
            return;
        }
        tracing::trace!(
            target: "agents",
            agency = %self.agency,
            consultants = consultants.len(),
            "monitor evaluation cycle observed Jev-capable agents"
        );
    }
}
