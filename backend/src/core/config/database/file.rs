use std::env;

use super::super::env_parse;
use super::super::system::SystemConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct PostgresConfig {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neo4jConnectionConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsStackConfig {
    pub enabled: bool,
    pub neo4j: Neo4jConnectionConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseConfigError {
    MissingVariable(&'static str),
    EmptyVariable(&'static str),
    InvalidEnabledValue(String),
}

impl std::fmt::Display for DatabaseConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingVariable(name) => write!(f, "missing environment variable {name}"),
            Self::EmptyVariable(name) => write!(f, "empty environment variable {name}"),
            Self::InvalidEnabledValue(raw) => {
                write!(f, "invalid BOT_AGENTS_ENABLED value: {raw}")
            }
        }
    }
}

impl std::error::Error for DatabaseConfigError {}

pub fn postgres_url_from_env() -> Result<Option<String>, DatabaseConfigError> {
    let _ = SystemConfig::active();
    match env::var("DATABASE_URL") {
        Ok(url) if url.trim().is_empty() => Err(DatabaseConfigError::EmptyVariable("DATABASE_URL")),
        Ok(url) => Ok(Some(url)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => {
            Err(DatabaseConfigError::EmptyVariable("DATABASE_URL"))
        }
    }
}

pub fn load_agents_stack_from_env() -> Result<AgentsStackConfig, DatabaseConfigError> {
    let defaults = SystemConfig::active();
    let enabled = parse_agents_enabled_flag(defaults.agents.enabled)?;
    if !enabled {
        return Ok(AgentsStackConfig {
            enabled: false,
            neo4j: Neo4jConnectionConfig {
                uri: String::new(),
                user: String::new(),
                password: String::new(),
                database: defaults.neo4j.database.clone(),
            },
        });
    }

    Ok(AgentsStackConfig {
        enabled: true,
        neo4j: Neo4jConnectionConfig {
            uri: env_or_toml("BOT_NEO4J_URI", &defaults.neo4j.uri)?,
            user: env_or_toml("BOT_NEO4J_USER", &defaults.neo4j.user)?,
            password: required_env("BOT_NEO4J_PASSWORD")?,
            database: env::var("BOT_NEO4J_DATABASE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| defaults.neo4j.database.clone()),
        },
    })
}

fn parse_agents_enabled_flag(toml_default: bool) -> Result<bool, DatabaseConfigError> {
    env_parse::parse_bool_flag("BOT_AGENTS_ENABLED", toml_default).map_err(|msg| {
        DatabaseConfigError::InvalidEnabledValue(
            msg.replace("invalid BOT_AGENTS_ENABLED value: ", ""),
        )
    })
}

fn env_or_toml(name: &'static str, toml: &str) -> Result<String, DatabaseConfigError> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Err(DatabaseConfigError::EmptyVariable(name)),
        Ok(value) => Ok(value),
        Err(_) if toml.trim().is_empty() => Err(DatabaseConfigError::MissingVariable(name)),
        Err(_) => Ok(toml.to_string()),
    }
}

pub fn graph_projection_outbox_drain_interval_secs() -> Option<u64> {
    if let Some(raw) = env_nonempty("BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_SECS") {
        let secs = raw.parse::<u64>().ok()?;
        return (secs > 0).then_some(secs);
    }
    let secs = SystemConfig::active()
        .neo4j
        .graph_projection_outbox_drain_secs;
    (secs > 0).then_some(secs)
}

pub fn graph_projection_outbox_drain_batch() -> u32 {
    if let Some(raw) = env_nonempty("BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_BATCH") {
        if let Ok(batch) = raw.parse::<u32>() {
            return batch.clamp(1, 500);
        }
    }
    32
}

fn env_nonempty(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn required_env(name: &'static str) -> Result<String, DatabaseConfigError> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Err(DatabaseConfigError::EmptyVariable(name)),
        Ok(value) => Ok(value),
        Err(_) => Err(DatabaseConfigError::MissingVariable(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_stack_disabled_by_default() {
        crate::core::test_env_lock::with_env_test_lock(|| {
            std::env::remove_var("BOT_AGENTS_ENABLED");
            let config = load_agents_stack_from_env().expect("load");
            assert!(!config.enabled);
        });
    }
}
