use std::sync::Arc;

use crate::core::persistence::Database;
use crate::core::providers::JevAdvisor;
use crate::modules::agents::AgentRegistry;
use crate::modules::config_api::Config;
use crate::modules::monitor::MonitorHandle;

#[derive(Clone)]
pub struct ApiState {
    inner: Arc<ApiStateInner>,
}

pub struct ApiStateInner {
    pub monitor: Option<MonitorHandle>,
    pub database: Option<Database>,
    pub agents: Arc<std::sync::Mutex<AgentRegistry>>,
    pub jev: Option<JevAdvisor>,
    pub app_config: Config,
}

impl ApiState {
    pub fn new(
        monitor: Option<MonitorHandle>,
        database: Option<Database>,
        jev: Option<JevAdvisor>,
        app_config: Config,
    ) -> Self {
        Self::with_agent_registry(
            monitor,
            database,
            jev,
            app_config,
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
        )
    }

    pub fn with_agent_registry(
        monitor: Option<MonitorHandle>,
        database: Option<Database>,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
    ) -> Self {
        Self {
            inner: Arc::new(ApiStateInner {
                monitor,
                database,
                agents,
                jev,
                app_config,
            }),
        }
    }

    pub fn monitor(&self) -> Option<&MonitorHandle> {
        self.inner.monitor.as_ref()
    }

    pub fn database(&self) -> Option<&Database> {
        self.inner.database.as_ref()
    }

    pub fn jev(&self) -> Option<&JevAdvisor> {
        self.inner.jev.as_ref()
    }

    pub fn app_config(&self) -> &Config {
        &self.inner.app_config
    }

    pub async fn with_agents<R>(&self, f: impl FnOnce(&mut AgentRegistry) -> R) -> R {
        let mut guard = self
            .inner
            .agents
            .lock()
            .expect("agent registry lock poisoned");
        f(&mut guard)
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new(
            None,
            None,
            None,
            crate::modules::config_api::Config::default(),
        )
    }
}
