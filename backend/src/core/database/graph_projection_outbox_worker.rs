//! Background ticker that drains `graph_projection_outbox` when PostgreSQL and Neo4j are wired (F2.1.2).

use std::time::Duration;

use sqlx::PgPool;
use tracing::{info, warn};

use crate::core::config::{
    graph_projection_outbox_drain_batch, graph_projection_outbox_drain_interval_secs,
};
use crate::core::persistence::Database;

use super::graph_projection_outbox::{drain_graph_projection_outbox, GraphProjectionOutboxError};
use super::neo4j::Neo4jGraph;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphProjectionOutboxStats {
    pub pending: u64,
    pub retry: u64,
    pub oldest_pending_age_secs: Option<u64>,
}

/// Signal for health JSON: backlog exists (does not fail readiness by itself).
pub fn graph_projection_outbox_degraded(stats: &GraphProjectionOutboxStats) -> bool {
    stats.pending > 0 || stats.retry > 0
}

pub async fn fetch_graph_projection_outbox_stats(
    pool: &PgPool,
) -> Result<GraphProjectionOutboxStats, GraphProjectionOutboxError> {
    let row: (i64, i64, Option<f64>) = sqlx::query_as(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'pending'),
            COUNT(*) FILTER (WHERE status = 'retry'),
            EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))
                FILTER (WHERE status IN ('pending', 'retry'))
         FROM graph_projection_outbox",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;

    let oldest_pending_age_secs = row.2.and_then(|secs| {
        if secs.is_finite() && secs >= 0.0 {
            Some(secs.round() as u64)
        } else {
            None
        }
    });

    Ok(GraphProjectionOutboxStats {
        pending: row.0 as u64,
        retry: row.1 as u64,
        oldest_pending_age_secs,
    })
}

pub fn spawn_graph_projection_outbox_drain_worker(postgres: Database, neo4j: Neo4jGraph) {
    let Some(interval_secs) = graph_projection_outbox_drain_interval_secs() else {
        return;
    };
    let batch = graph_projection_outbox_drain_batch();
    let pool = postgres.pool().clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        info!(
            target: "database",
            interval_secs,
            batch,
            "graph projection outbox background drain enabled"
        );
        loop {
            ticker.tick().await;
            match drain_graph_projection_outbox(&pool, &neo4j, batch).await {
                Ok(summary) if summary.processed > 0 => {
                    info!(
                        target: "database",
                        processed = summary.processed,
                        succeeded = summary.succeeded,
                        failed = summary.failed,
                        "graph projection outbox drain tick"
                    );
                }
                Err(error) => {
                    warn!(
                        target: "database",
                        %error,
                        "graph projection outbox drain tick failed (rows remain pending/retry)"
                    );
                }
                _ => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn degraded_when_pending_or_retry_positive() {
        assert!(!graph_projection_outbox_degraded(
            &GraphProjectionOutboxStats::default()
        ));
        assert!(graph_projection_outbox_degraded(
            &GraphProjectionOutboxStats {
                pending: 1,
                retry: 0,
                oldest_pending_age_secs: None,
            }
        ));
    }

    #[test]
    fn drain_interval_parses_positive_seconds() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_SECS", "45");
            assert_eq!(graph_projection_outbox_drain_interval_secs(), Some(45));
            std::env::remove_var("BOT_GRAPH_PROJECTION_OUTBOX_DRAIN_SECS");
        });
    }

    #[tokio::test]
    async fn fetch_stats_returns_zeros_on_empty_outbox() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let stats = fetch_graph_projection_outbox_stats(db.pool())
            .await
            .expect("stats");
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.retry, 0);
    }
}
