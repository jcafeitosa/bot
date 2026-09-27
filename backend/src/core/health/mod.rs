//! Process liveness and dependency readiness probes (no domain policy).

use crate::core::persistence::{Database, PersistenceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeStatus {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentProbe {
    pub name: &'static str,
    pub status: ProbeStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessReport {
    pub ready: bool,
    pub components: Vec<ComponentProbe>,
}

/// Liveness: the process accepted the check (no external I/O).
pub fn liveness() -> ProbeStatus {
    ProbeStatus::Up
}

/// Readiness: optional PostgreSQL when a [`Database`] handle is wired (e.g. HTTP API).
pub async fn readiness(database: Option<&Database>) -> ReadinessReport {
    let mut components = vec![ComponentProbe {
        name: "process",
        status: ProbeStatus::Up,
        detail: None,
    }];

    if let Some(db) = database {
        let probe = match db.ping().await {
            Ok(()) => ComponentProbe {
                name: "database",
                status: ProbeStatus::Up,
                detail: Some("ok".into()),
            },
            Err(error) => ComponentProbe {
                name: "database",
                status: ProbeStatus::Down,
                detail: Some(readiness_error_detail(&error)),
            },
        };
        components.push(probe);
    }

    let ready = components
        .iter()
        .all(|component| component.status == ProbeStatus::Up);
    ReadinessReport { ready, components }
}

fn readiness_error_detail(error: &PersistenceError) -> String {
    match error {
        PersistenceError::Connect(_) => "connection failed".into(),
        PersistenceError::MissingUrl => "database url missing".into(),
        PersistenceError::WrongDatabase => "wrong database name".into(),
        PersistenceError::InvalidUrl => "invalid database url".into(),
        PersistenceError::Migrate(_) => "migration failed".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liveness_is_always_up() {
        assert_eq!(liveness(), ProbeStatus::Up);
    }

    #[tokio::test]
    async fn readiness_without_database_only_checks_process() {
        let report = readiness(None).await;
        assert!(report.ready);
        assert_eq!(report.components.len(), 1);
        assert_eq!(report.components[0].name, "process");
    }
}
