use std::env;

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
    let enabled = parse_enabled_flag()?;
    if !enabled {
        return Ok(AgentsStackConfig {
            enabled: false,
            neo4j: Neo4jConnectionConfig {
                uri: String::new(),
                user: String::new(),
                password: String::new(),
                database: "neo4j".into(),
            },
        });
    }

    Ok(AgentsStackConfig {
        enabled: true,
        neo4j: Neo4jConnectionConfig {
            uri: required_nonempty("BOT_NEO4J_URI")?,
            user: required_nonempty("BOT_NEO4J_USER")?,
            password: required_nonempty("BOT_NEO4J_PASSWORD")?,
            database: env::var("BOT_NEO4J_DATABASE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "neo4j".into()),
        },
    })
}

fn parse_enabled_flag() -> Result<bool, DatabaseConfigError> {
    match env::var("BOT_AGENTS_ENABLED") {
        Ok(raw) => {
            let normalized = raw.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "1" | "true" | "yes" | "on" => Ok(true),
                "0" | "false" | "no" | "off" | "" => Ok(false),
                other => Err(DatabaseConfigError::InvalidEnabledValue(other.to_string())),
            }
        }
        Err(_) => Ok(false),
    }
}

fn required_nonempty(name: &'static str) -> Result<String, DatabaseConfigError> {
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
    fn agents_stack_disabled_when_flag_absent() {
        std::env::remove_var("BOT_AGENTS_ENABLED");
        let config = load_agents_stack_from_env().expect("load");
        assert!(!config.enabled);
    }
}
