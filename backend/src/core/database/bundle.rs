use tracing::warn;

use crate::core::persistence::{Database, PersistenceError};

use super::monitor_bootstrap::{bootstrap_monitor_postgres, MonitorBootstrapError};

use super::config::{load_agents_stack_from_env, postgres_url_from_env};
use super::neo4j::Neo4jGraph;

#[derive(Clone, Default)]
pub struct AppDatabases {
    pub postgres: Option<Database>,
    pub neo4j: Option<Neo4jGraph>,
}

impl std::fmt::Debug for AppDatabases {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppDatabases")
            .field("postgres", &self.postgres.is_some())
            .field("neo4j", &self.neo4j.is_some())
            .finish()
    }
}

impl AppDatabases {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn postgres_handle(&self) -> Option<&Database> {
        self.postgres.as_ref()
    }

    pub fn neo4j(&self) -> Option<&Neo4jGraph> {
        self.neo4j.as_ref()
    }

    pub async fn bootstrap_runtime() -> Self {
        Self::bootstrap_http_api().await
    }

    pub async fn bootstrap_http_api() -> Self {
        let postgres = match postgres_url_from_env() {
            Ok(Some(url)) => match Database::connect_from_url(&url).await {
                Ok(db) => match db.migrate().await {
                    Ok(()) => Some(db),
                    Err(error) => {
                        warn!(target: "database", %error, "PostgreSQL migrations failed");
                        None
                    }
                },
                Err(error) => {
                    warn!(target: "database", %error, "DATABASE_URL present but connection failed");
                    None
                }
            },
            Ok(None) => None,
            Err(error) => {
                warn!(target: "database", %error, "invalid DATABASE_URL configuration");
                None
            }
        };

        let neo4j = match load_agents_stack_from_env() {
            Ok(config) if config.enabled => match Neo4jGraph::connect(&config.neo4j).await {
                Ok(graph) => Some(graph),
                Err(error) => {
                    warn!(target: "database", %error, "Neo4j enabled but connection failed");
                    None
                }
            },
            Ok(_) => None,
            Err(error) => {
                warn!(target: "database", %error, "invalid agents stack configuration");
                None
            }
        };

        Self { postgres, neo4j }
    }

    pub async fn bootstrap_monitor_postgres(
        persist_flag: Option<&str>,
        timeframe: &str,
    ) -> Result<Option<Database>, MonitorBootstrapError> {
        bootstrap_monitor_postgres(persist_flag, timeframe).await
    }

    pub async fn postgres_for_cli_persist() -> Result<Database, PersistenceError> {
        super::monitor_bootstrap::postgres_for_cli_persist().await
    }
}
