//! Operational system defaults (`system.toml`) merged with `.env` (env wins).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::core::config::load::parse_embedded_toml;
use crate::core::error::{BotError, BotResult};

pub const DEFAULT_SYSTEM_CONFIG_PATH: &str = "src/core/config/system.toml";

static ACTIVE: OnceLock<SystemConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SystemConfig {
    #[serde(default)]
    pub postgres: PostgresSection,
    #[serde(default)]
    pub agents: AgentsSection,
    #[serde(default)]
    pub neo4j: Neo4jSection,
    #[serde(default)]
    pub monitor: MonitorSection,
    #[serde(default)]
    pub orders: OrdersSection,
    #[serde(default)]
    pub bots: BotsSection,
    #[serde(default)]
    pub http: HttpSection,
    #[serde(default)]
    pub providers: ProvidersSection,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PostgresSection {}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentsSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub monitor_agency: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Neo4jSection {
    #[serde(default = "default_neo4j_uri")]
    pub uri: String,
    #[serde(default = "default_neo4j_user")]
    pub user: String,
    #[serde(default = "default_neo4j_database")]
    pub database: String,
}

fn default_neo4j_uri() -> String {
    "bolt://127.0.0.1:7688".into()
}
fn default_neo4j_user() -> String {
    "neo4j".into()
}
fn default_neo4j_database() -> String {
    "neo4j".into()
}

impl Default for Neo4jSection {
    fn default() -> Self {
        Self {
            uri: default_neo4j_uri(),
            user: default_neo4j_user(),
            database: default_neo4j_database(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MonitorSection {
    #[serde(default)]
    pub persist_market_data: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OrdersSection {
    #[serde(default)]
    pub execution: String,
    #[serde(default)]
    pub exchange_submit: String,
    #[serde(default)]
    pub paper_fill_unit_price: f64,
    #[serde(default)]
    pub reconciliation_poll_secs: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BotsSection {
    #[serde(default)]
    pub runtime_enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct HttpSection {
    #[serde(default)]
    #[allow(dead_code)]
    pub admin: HttpAdminSection,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HttpAdminSection {}

#[derive(Debug, Clone, Deserialize)]
pub struct ProvidersSection {
    #[serde(default = "default_typesafe_endpoint")]
    pub typesafe_endpoint: String,
    #[serde(default)]
    pub openai_base_url: String,
    #[serde(default)]
    pub nim_base_url: String,
}

fn default_typesafe_endpoint() -> String {
    "https://api.typesafe.ai/v1/systemone".into()
}

impl Default for ProvidersSection {
    fn default() -> Self {
        Self {
            typesafe_endpoint: default_typesafe_endpoint(),
            openai_base_url: String::new(),
            nim_base_url: String::new(),
        }
    }
}

impl SystemConfig {
    pub fn bundled() -> Self {
        parse_embedded_toml(include_str!("../system.toml"), "system")
    }

    pub fn load(path: &Path) -> BotResult<Self> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            BotError::Configuration(format!("cannot read system config {}: {e}", path.display()))
        })?;
        toml::from_str(&raw)
            .map_err(|e| BotError::Configuration(format!("invalid system TOML: {e}")))
    }

    pub fn init_from_path(path: &Path) -> BotResult<()> {
        let cfg = if path.is_file() {
            Self::load(path)?
        } else {
            Self::bundled()
        };
        ACTIVE.get_or_init(|| cfg);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn init_default_path() -> BotResult<()> {
        Self::init_from_path(Path::new(DEFAULT_SYSTEM_CONFIG_PATH))
    }

    pub fn active() -> &'static SystemConfig {
        ACTIVE
            .get()
            .unwrap_or_else(|| ACTIVE.get_or_init(SystemConfig::bundled))
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from(DEFAULT_SYSTEM_CONFIG_PATH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_system_toml_parses() {
        let cfg = SystemConfig::bundled();
        assert!(!cfg.monitor.persist_market_data);
        assert_eq!(cfg.neo4j.user, "neo4j");
    }
}
