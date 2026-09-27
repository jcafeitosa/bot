use std::sync::{Arc, Mutex};

use crate::modules::agents::controllers::registry::AgentRegistry;
use crate::modules::agents::controllers::supervisor_hook::{
    MonitorAgentHook, NoopMonitorAgentHook, RegistryMonitorAgentHook,
};
use crate::modules::agents::models::AgencyId;

/// Composition-root helper for the trading monitor: binds an in-memory registry when `BOT_AGENCY` is set.
pub fn monitor_agent_hook_from_env() -> Arc<dyn MonitorAgentHook> {
    match std::env::var("BOT_AGENCY") {
        Ok(raw) if !raw.trim().is_empty() => match AgencyId::new(raw.trim()) {
            Ok(agency) => {
                tracing::info!(
                    target: "agents",
                    agency = %agency,
                    "Monitor agent hook bound to agency (in-memory registry)"
                );
                Arc::new(RegistryMonitorAgentHook::new(
                    Arc::new(Mutex::new(AgentRegistry::new())),
                    agency,
                ))
            }
            Err(error) => {
                tracing::warn!(
                    target: "agents",
                    error = %error,
                    "Invalid BOT_AGENCY; monitor uses noop agent hook"
                );
                Arc::new(NoopMonitorAgentHook)
            }
        },
        _ => Arc::new(NoopMonitorAgentHook),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_agency_falls_back_to_noop() {
        std::env::set_var("BOT_AGENCY", "!!!");
        let hook = monitor_agent_hook_from_env();
        std::env::remove_var("BOT_AGENCY");
        assert!(hook.evaluation_agents().is_empty());
    }

    #[test]
    fn valid_agency_returns_registry_hook() {
        std::env::set_var("BOT_AGENCY", "acme");
        let hook = monitor_agent_hook_from_env();
        std::env::remove_var("BOT_AGENCY");
        assert!(hook.evaluation_agents().is_empty());
    }
}
