//! Process liveness and dependency readiness probes (no domain policy).

use crate::core::database::{AppDatabases, DatabaseError, PostgresDatabase};
use crate::core::persistence::Database;

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

pub fn liveness() -> ProbeStatus {
    ProbeStatus::Up
}

pub async fn readiness_databases(databases: &AppDatabases) -> ReadinessReport {
    let mut components = vec![ComponentProbe {
        name: "process",
        status: ProbeStatus::Up,
        detail: None,
    }];

    if let Some(db) = databases.postgres_handle() {
        let pg = db.as_postgres();
        components.push(postgres_probe(pg).await);
        components.push(postgres_extensions_probe(pg).await);
    }

    if let Some(graph) = databases.neo4j() {
        let probe = match graph.ping().await {
            Ok(()) => ComponentProbe {
                name: "neo4j",
                status: ProbeStatus::Up,
                detail: Some("ok".into()),
            },
            Err(_) => ComponentProbe {
                name: "neo4j",
                status: ProbeStatus::Down,
                detail: Some("graph connection failed".into()),
            },
        };
        components.push(probe);
    }

    finish_report(components)
}

#[allow(dead_code)]
pub async fn readiness(database: Option<&Database>) -> ReadinessReport {
    let mut components = vec![ComponentProbe {
        name: "process",
        status: ProbeStatus::Up,
        detail: None,
    }];

    if let Some(db) = database {
        let pg = db.as_postgres();
        components.push(postgres_probe(pg).await);
        components.push(postgres_extensions_probe(pg).await);
    }

    finish_report(components)
}

fn finish_report(components: Vec<ComponentProbe>) -> ReadinessReport {
    let ready = components
        .iter()
        .all(|component| component.status == ProbeStatus::Up);
    ReadinessReport { ready, components }
}

async fn postgres_probe(pg: &PostgresDatabase) -> ComponentProbe {
    match pg.ping().await {
        Ok(()) => ComponentProbe {
            name: "postgres",
            status: ProbeStatus::Up,
            detail: Some("ok".into()),
        },
        Err(error) => ComponentProbe {
            name: "postgres",
            status: ProbeStatus::Down,
            detail: Some(readiness_error_detail(&error)),
        },
    }
}

async fn postgres_extensions_probe(pg: &PostgresDatabase) -> ComponentProbe {
    match pg.extension_health().await {
        Ok(health) if health.ok => ComponentProbe {
            name: "postgres_extensions",
            status: ProbeStatus::Up,
            detail: Some("timescaledb,vector".into()),
        },
        Ok(health) => ComponentProbe {
            name: "postgres_extensions",
            status: ProbeStatus::Down,
            detail: Some(format!("missing: {}", health.missing.join(","))),
        },
        Err(error) => ComponentProbe {
            name: "postgres_extensions",
            status: ProbeStatus::Down,
            detail: Some(readiness_error_detail(&error)),
        },
    }
}

fn readiness_error_detail(error: &DatabaseError) -> String {
    match error {
        DatabaseError::Connect(_) => "connection failed".into(),
        DatabaseError::MissingUrl => "database url missing".into(),
        DatabaseError::WrongDatabase => "wrong database name".into(),
        DatabaseError::InvalidUrl => "invalid database url".into(),
        DatabaseError::Migrate(_) => "migration failed".into(),
        DatabaseError::UnsupportedVersion => "postgresql 18+ required".into(),
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
    }

    #[tokio::test]
    async fn readiness_databases_empty_only_checks_process() {
        let report = readiness_databases(&AppDatabases::empty()).await;
        assert!(report.ready);
        assert_eq!(report.components.len(), 1);
    }
}
