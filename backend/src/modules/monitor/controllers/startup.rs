#![allow(dead_code)] // injectable bootstrap API exercised by unit tests

use std::future::Future;

use crate::core::database::MonitorBootstrapError;
use crate::core::persistence::Database;

pub type StartupError = MonitorBootstrapError;

pub trait MonitorStore: Sized {
    async fn migrate(&self) -> Result<(), StartupError>;
}

impl MonitorStore for Database {
    async fn migrate(&self) -> Result<(), StartupError> {
        Database::migrate(self)
            .await
            .map_err(|_| StartupError::Migration)
    }
}

pub fn persistence_required(value: Option<&str>) -> Result<bool, StartupError> {
    crate::core::database::monitor_bootstrap::persistence_required(value)
}

pub async fn bootstrap_monitor<T, FUrl, FConnect, Fut>(
    flag: Option<&str>,
    timeframe: &str,
    read_url: FUrl,
    connect: FConnect,
) -> Result<Option<T>, StartupError>
where
    T: MonitorStore,
    FUrl: FnOnce() -> Result<Option<String>, StartupError>,
    FConnect: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<T, StartupError>>,
{
    if !persistence_required(flag)? {
        return Ok(None);
    }
    if timeframe != "1m" {
        return Err(StartupError::UnsupportedTimeframe);
    }
    let url = read_url()?.ok_or(StartupError::MissingUrl)?;
    if url.is_empty() {
        return Err(StartupError::InvalidUrl);
    }
    let database = connect(url).await?;
    database.migrate().await?;
    Ok(Some(database))
}

pub async fn connect_database(url: String) -> Result<Database, StartupError> {
    crate::core::database::connect_postgres_for_monitor(url).await
}

pub async fn bootstrap_monitor_database(
    persist_flag: Option<&str>,
    timeframe: &str,
) -> Result<Option<Database>, StartupError> {
    bootstrap_monitor(
        persist_flag,
        timeframe,
        || {
            crate::core::config::monitor::database_url_for_monitor()
                .map_err(|_| StartupError::InvalidUrl)
        },
        connect_database,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug)]
    struct FakeStore {
        calls: Arc<Mutex<Vec<&'static str>>>,
        fail_migrate: bool,
    }

    impl MonitorStore for FakeStore {
        async fn migrate(&self) -> Result<(), StartupError> {
            self.calls.lock().unwrap().push("migrate");
            if self.fail_migrate {
                Err(StartupError::Migration)
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn off_does_not_read_url_or_connect() {
        for flag in [None, Some("0"), Some("false"), Some("FALSE")] {
            let result = bootstrap_monitor::<FakeStore, _, _, _>(
                flag,
                "1m",
                || panic!("DATABASE_URL must not be read"),
                |_| async { panic!("database must not be connected") },
            )
            .await;
            assert!(result.unwrap().is_none());
        }
    }

    #[tokio::test]
    async fn invalid_flag_fails_before_url_access() {
        let result = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("yes"),
            "1m",
            || panic!("DATABASE_URL must not be read"),
            |_| async { panic!("database must not be connected") },
        )
        .await;
        assert!(matches!(result, Err(StartupError::InvalidFlag)));
    }

    #[tokio::test]
    async fn required_rejects_wrong_timeframe_before_url_access() {
        let result = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("true"),
            "5m",
            || panic!("DATABASE_URL must not be read"),
            |_| async { panic!("database must not be connected") },
        )
        .await;
        assert!(matches!(result, Err(StartupError::UnsupportedTimeframe)));
    }

    #[tokio::test]
    async fn required_rejects_missing_or_empty_url() {
        let missing = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("1"),
            "1m",
            || Ok(None),
            |_| async { panic!("database must not be connected") },
        )
        .await;
        assert!(matches!(missing, Err(StartupError::MissingUrl)));

        let empty = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("1"),
            "1m",
            || Ok(Some(String::new())),
            |_| async { panic!("database must not be connected") },
        )
        .await;
        assert!(matches!(empty, Err(StartupError::InvalidUrl)));

        let non_unicode = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("1"),
            "1m",
            || Err(StartupError::InvalidUrl),
            |_| async { panic!("database must not be connected") },
        )
        .await;
        assert!(matches!(non_unicode, Err(StartupError::InvalidUrl)));
    }

    #[tokio::test]
    async fn required_connects_and_migrates_once() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let connected_calls = calls.clone();
        let result = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("TRUE"),
            "1m",
            || Ok(Some("postgresql://localhost/trading_bot".to_owned())),
            move |_| async move {
                connected_calls.lock().unwrap().push("connect");
                Ok(FakeStore {
                    calls: connected_calls,
                    fail_migrate: false,
                })
            },
        )
        .await;
        assert!(result.unwrap().is_some());
        assert_eq!(*calls.lock().unwrap(), ["connect", "migrate"]);
    }

    #[tokio::test]
    async fn required_connection_failure_stops_before_migration() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let connected_calls = calls.clone();
        let result = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("1"),
            "1m",
            || Ok(Some("private-url".to_owned())),
            move |_| async move {
                connected_calls.lock().unwrap().push("connect");
                Err(StartupError::Connection)
            },
        )
        .await;
        assert!(matches!(result, Err(StartupError::Connection)));
        assert_eq!(*calls.lock().unwrap(), ["connect"]);
    }

    #[tokio::test]
    async fn required_migration_failure_is_sanitized() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let connected_calls = calls.clone();
        let result = bootstrap_monitor::<FakeStore, _, _, _>(
            Some("1"),
            "1m",
            || Ok(Some("private-url".to_owned())),
            move |_| async move {
                connected_calls.lock().unwrap().push("connect");
                Ok(FakeStore {
                    calls: connected_calls,
                    fail_migrate: true,
                })
            },
        )
        .await;
        let message = result.unwrap_err().to_string();
        assert_eq!(message, "PostgreSQL migration failed");
        assert!(!message.contains("private-url"));
        assert_eq!(*calls.lock().unwrap(), ["connect", "migrate"]);
    }

    #[tokio::test]
    async fn real_connection_parser_rejects_bad_url_and_wrong_database_without_network() {
        assert!(matches!(
            connect_database("not a url".to_owned()).await,
            Err(StartupError::InvalidUrl)
        ));
        assert!(matches!(
            connect_database("postgresql://localhost/other".to_owned()).await,
            Err(StartupError::WrongDatabase)
        ));
    }
}
