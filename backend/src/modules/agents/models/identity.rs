use std::fmt;

use thiserror::Error;

const MAX_LABEL_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgencyId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnerId(String);

impl AgentId {
    pub fn new(raw: impl AsRef<str>) -> Result<Self, AgentsError> {
        Ok(Self(normalize_id(raw, "agent id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AgencyId {
    pub fn new(raw: impl AsRef<str>) -> Result<Self, AgentsError> {
        Ok(Self(normalize_id(raw, "agency id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl OwnerId {
    pub fn new(raw: impl AsRef<str>) -> Result<Self, AgentsError> {
        Ok(Self(normalize_id(raw, "owner id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalize_id(raw: impl AsRef<str>, label: &str) -> Result<String, AgentsError> {
    let trimmed = raw.as_ref().trim();
    if trimmed.is_empty() {
        return Err(AgentsError::InvalidId(format!("{label} cannot be empty")));
    }
    if trimmed.len() > MAX_LABEL_LEN {
        return Err(AgentsError::InvalidId(format!(
            "{label} exceeds {MAX_LABEL_LEN} characters"
        )));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AgentsError::InvalidId(format!(
            "{label} contains control characters"
        )));
    }
    Ok(trimmed.to_owned())
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for AgencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for OwnerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AgentsError {
    #[error("invalid agent identity: {0}")]
    InvalidId(String),
    #[error("agent not found: {0}")]
    NotFound(String),
    #[error("agency mismatch for agent {agent}")]
    AgencyMismatch { agent: String },
    #[error("hierarchy violation: {0}")]
    Hierarchy(String),
    #[error("invalid lifecycle transition: {0}")]
    Lifecycle(String),
    #[error("advisory not permitted: {0}")]
    AdvisoryDenied(String),
    #[error("duplicate agent id: {0}")]
    Duplicate(String),
    #[error("identity persistence failed: {0}")]
    Persistence(String),
}
