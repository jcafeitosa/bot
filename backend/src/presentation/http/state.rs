use std::sync::Arc;

use crate::core::database::AppDatabases;
use crate::core::persistence::Database;
use crate::core::providers::JevAdvisor;
use crate::modules::agents::AgentRegistry;
use crate::modules::bots::BotCatalogBackend;
use crate::modules::config_api::Config;
use crate::modules::monitor::MonitorHandle;

#[derive(Clone)]
pub struct ApiState {
    inner: Arc<ApiStateInner>,
}

pub struct ApiStateInner {
    pub monitor: Option<MonitorHandle>,
    pub databases: AppDatabases,
    pub agents: Arc<std::sync::Mutex<AgentRegistry>>,
    pub bot_catalog: Arc<tokio::sync::Mutex<BotCatalogBackend>>,
    pub jev: Option<JevAdvisor>,
    pub app_config: Config,
}

impl ApiState {
    pub fn new(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            Arc::new(std::sync::Mutex::new(AgentRegistry::new())),
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
        )
    }

    pub fn with_agent_registry(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
    ) -> Self {
        Self::with_stores(
            monitor,
            databases.clone(),
            jev,
            app_config,
            agents,
            Arc::new(tokio::sync::Mutex::new(BotCatalogBackend::from_databases(
                &databases,
            ))),
        )
    }

    pub fn with_stores(
        monitor: Option<MonitorHandle>,
        databases: AppDatabases,
        jev: Option<JevAdvisor>,
        app_config: Config,
        agents: Arc<std::sync::Mutex<AgentRegistry>>,
        bot_catalog: Arc<tokio::sync::Mutex<BotCatalogBackend>>,
    ) -> Self {
        Self {
            inner: Arc::new(ApiStateInner {
                monitor,
                databases,
                agents,
                bot_catalog,
                jev,
                app_config,
            }),
        }
    }

    pub fn monitor(&self) -> Option<&MonitorHandle> {
        self.inner.monitor.as_ref()
    }

    pub fn databases(&self) -> &AppDatabases {
        &self.inner.databases
    }

    pub fn database(&self) -> Option<&Database> {
        self.inner.databases.postgres_handle()
    }

    pub fn jev(&self) -> Option<&JevAdvisor> {
        self.inner.jev.as_ref()
    }

    pub fn app_config(&self) -> &Config {
        &self.inner.app_config
    }

    pub fn bot_catalog(&self) -> &Arc<tokio::sync::Mutex<BotCatalogBackend>> {
        &self.inner.bot_catalog
    }

    pub async fn with_bot_catalog<F, Fut, R>(&self, f: F) -> R
    where
        F: for<'a> FnOnce(&'a mut BotCatalogBackend) -> Fut,
        Fut: std::future::Future<Output = R> + Send,
        R: Send,
    {
        let mut guard = self.inner.bot_catalog.lock().await;
        f(&mut guard).await
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
            AppDatabases::empty(),
            None,
            crate::modules::config_api::Config::default(),
        )
    }
}
