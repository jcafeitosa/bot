use std::env;

use super::super::database::postgres_url_from_env;
use super::super::system::SystemConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorEnvError {
    InvalidFlag,
    InvalidUrl,
}

impl std::fmt::Display for MonitorEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFlag => write!(f, "invalid PERSIST_MARKET_DATA"),
            Self::InvalidUrl => write!(f, "invalid DATABASE_URL"),
        }
    }
}

impl std::error::Error for MonitorEnvError {}

pub fn persist_market_data_flag_raw() -> Result<Option<String>, MonitorEnvError> {
    match env::var("PERSIST_MARKET_DATA") {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => {
            let cfg = SystemConfig::active();
            if cfg.monitor.persist_market_data {
                Ok(Some("true".into()))
            } else {
                Ok(None)
            }
        }
        Err(env::VarError::NotUnicode(_)) => Err(MonitorEnvError::InvalidFlag),
    }
}

pub fn database_url_for_monitor() -> Result<Option<String>, MonitorEnvError> {
    postgres_url_from_env().map_err(|_| MonitorEnvError::InvalidUrl)
}
