use std::sync::{Arc, Mutex, OnceLock};

use crate::modules::agents::controllers::registry::AgentRegistry;
use crate::modules::agents::controllers::supervisor_hook::{
    MonitorAgentHook, NoopMonitorAgentHook, RegistryMonitorAgentHook,
};
use crate::modules::agents::models::AgencyId;

/// Process-wide in-memory agent registry (HTTP API and monitor hook share this when both run in the same process).
pub fn shared_agent_registry() -> Arc<Mutex<AgentRegistry>> {
    static REGISTRY: OnceLock<Arc<Mutex<AgentRegistry>>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| Arc::new(Mutex::new(AgentRegistry::new())))
        .clone()
}

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
                    shared_agent_registry(),
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

    #[test]
    fn monitor_hook_shares_process_registry_with_api() {
        use crate::modules::agents::models::{
            AgentCapabilities, AgentId, AgentRole, NewAgentSpec, OwnerId, SupervisorRef,
        };
        use crate::modules::agents::{MonitorAgentHook, RegistryMonitorAgentHook};

        let agency =
            crate::modules::agents::models::AgencyId::new("hook-shared-registry-agency").unwrap();
        let hook = RegistryMonitorAgentHook::new(shared_agent_registry(), agency.clone());
        let spec = NewAgentSpec {
            id: AgentId::new("jev-analyst-shared").unwrap(),
            agency: agency.clone(),
            owner: OwnerId::new("owner-1").unwrap(),
            display_name: "Analyst".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(OwnerId::new("owner-1").unwrap()),
            capabilities: AgentCapabilities { consult_jev: true },
        };
        {
            let registry = shared_agent_registry();
            let mut reg = registry.lock().unwrap();
            if reg.get(&agency, &spec.id).is_err() {
                reg.register(spec.clone(), 1).unwrap();
            }
        }

        let ids = hook.evaluation_agents();
        assert!(
            ids.iter().any(|id| id.as_str() == "jev-analyst-shared"),
            "expected shared registry agent in hook evaluation list, got {:?}",
            ids
        );
    }

    #[test]
    fn monitor_agent_hook_from_env_shares_registry() {
        use std::sync::Mutex;

        use crate::modules::agents::models::{
            AgentCapabilities, AgentId, AgentRole, NewAgentSpec, OwnerId, SupervisorRef,
        };
        use crate::modules::agents::MonitorAgentHook;

        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();

        let agency_raw = "env-hook-shared-agency";
        std::env::set_var("BOT_AGENCY", agency_raw);
        let hook = monitor_agent_hook_from_env();
        let agency = crate::modules::agents::models::AgencyId::new(agency_raw).unwrap();
        let spec = NewAgentSpec {
            id: AgentId::new("env-hook-analyst").unwrap(),
            agency: agency.clone(),
            owner: OwnerId::new("owner-1").unwrap(),
            display_name: "Analyst".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(OwnerId::new("owner-1").unwrap()),
            capabilities: AgentCapabilities { consult_jev: true },
        };
        {
            let registry = shared_agent_registry();
            let mut reg = registry.lock().unwrap();
            if reg.get(&agency, &spec.id).is_err() {
                reg.register(spec, 1).unwrap();
            }
        }
        let ids = hook.evaluation_agents();
        std::env::remove_var("BOT_AGENCY");
        assert!(ids.iter().any(|id| id.as_str() == "env-hook-analyst"));
    }
}
