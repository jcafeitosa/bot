use thiserror::Error;

use crate::core::persistence::{Database, PersistenceError};

use super::config::postgres_url_from_env;
use crate::core::config::monitor::database_url_for_monitor;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MonitorBootstrapError {
    #[error("PERSIST_MARKET_DATA must be 0, 1, false, or true")]
    InvalidFlag,
    #[error("PERSIST_MARKET_DATA requires market.timeframe=1m")]
    UnsupportedTimeframe,
    #[error("DATABASE_URL is required when PERSIST_MARKET_DATA is enabled")]
    MissingUrl,
    #[error("DATABASE_URL is invalid")]
    InvalidUrl,
    #[error("DATABASE_URL must point to the dedicated trading_bot database")]
    WrongDatabase,
    #[error("PostgreSQL connection or health check failed")]
    Connection,
    #[error("PostgreSQL migration failed")]
    Migration,
}

pub fn persistence_required(value: Option<&str>) -> Result<bool, MonitorBootstrapError> {
    match value {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        Some(value) if value.eq_ignore_ascii_case("false") => Ok(false),
        Some(value) if value.eq_ignore_ascii_case("true") => Ok(true),
        Some(_) => Err(MonitorBootstrapError::InvalidFlag),
    }
}

pub async fn connect_postgres_for_monitor(url: String) -> Result<Database, MonitorBootstrapError> {
    Database::connect_from_url(&url)
        .await
        .map_err(|error| match error {
            PersistenceError::WrongDatabase => MonitorBootstrapError::WrongDatabase,
            PersistenceError::InvalidUrl => MonitorBootstrapError::InvalidUrl,
            _ => MonitorBootstrapError::Connection,
        })
}

pub async fn bootstrap_monitor_postgres(
    persist_flag: Option<&str>,
    timeframe: &str,
) -> Result<Option<Database>, MonitorBootstrapError> {
    if !persistence_required(persist_flag)? {
        return Ok(None);
    }
    if timeframe != "1m" {
        return Err(MonitorBootstrapError::UnsupportedTimeframe);
    }
    let url = database_url_for_monitor()
        .map_err(|_| MonitorBootstrapError::InvalidUrl)?
        .filter(|value| !value.is_empty())
        .ok_or(MonitorBootstrapError::MissingUrl)?;
    let database = connect_postgres_for_monitor(url).await?;
    database
        .migrate()
        .await
        .map_err(|_| MonitorBootstrapError::Migration)?;
    Ok(Some(database))
}

pub async fn postgres_for_cli_persist() -> Result<Database, PersistenceError> {
    let url = postgres_url_from_env()
        .map_err(|_| PersistenceError::InvalidUrl)?
        .filter(|value| !value.trim().is_empty())
        .ok_or(PersistenceError::MissingUrl)?;
    if url.trim().is_empty() {
        return Err(PersistenceError::InvalidUrl);
    }
    let db = Database::connect_from_url(&url).await?;
    db.migrate().await?;
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_required_accepts_common_flags() {
        assert!(!persistence_required(None).unwrap());
        assert!(!persistence_required(Some("0")).unwrap());
        assert!(persistence_required(Some("1")).unwrap());
        assert!(!persistence_required(Some("false")).unwrap());
        assert!(persistence_required(Some("true")).unwrap());
        assert_eq!(
            persistence_required(Some("maybe")),
            Err(MonitorBootstrapError::InvalidFlag)
        );
    }
}
