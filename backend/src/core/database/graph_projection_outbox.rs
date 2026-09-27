//! Durable PG outbox for Neo4j graph projection (F2.1).
//!
//! **Enqueue timing:** hooks run after PG commit today (`persist_agent_after_mutation`,
//! catalog persist, order submit). Rows are inserted in a separate statement — not yet in the
//! same transaction as domain writes. Orders HTTP (PG claim path) uses
//! `persist_idempotency_and_enqueue_graph_projection` + `enqueue_graph_projection_outbox_tx`.

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;

use super::graph_projection::{
    AgentHierarchyProjection, BotCatalogProjection, BotPromotionProjection, GraphProjectionError,
    GraphProjectionPort, OrderIntentProjection, SubmittedEdgeProjection, AGENTS_GRAPH_DOMAIN,
    BOTS_GRAPH_DOMAIN, TRADING_GRAPH_DOMAIN,
};
use super::neo4j::Neo4jGraph;
use super::postgres::PostgresDatabase;

#[derive(Debug, Clone, Copy)]
pub struct GraphProjectionSync<'a> {
    pub postgres: Option<&'a PostgresDatabase>,
    pub neo4j: Option<&'a Neo4jGraph>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphProjectionPayload {
    AgentHierarchy(AgentHierarchyProjection),
    BotCatalog(BotCatalogProjection),
    BotPromotion(BotPromotionProjection),
    OrderIntent(OrderIntentProjection),
    SubmittedEdge(SubmittedEdgeProjection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphProjectionOutboxMessage {
    pub graph_domain: String,
    pub event_kind: String,
    pub idempotency_key: String,
    pub payload: GraphProjectionPayload,
}

impl GraphProjectionOutboxMessage {
    pub fn agent_hierarchy(projection: AgentHierarchyProjection) -> Self {
        let idempotency_key = format!("agent:{}:{}", projection.agency_id, projection.agent_id);
        Self {
            graph_domain: AGENTS_GRAPH_DOMAIN.to_string(),
            event_kind: "agent_hierarchy".to_string(),
            idempotency_key,
            payload: GraphProjectionPayload::AgentHierarchy(projection),
        }
    }

    pub fn bot_catalog(projection: BotCatalogProjection) -> Self {
        let idempotency_key = format!("bot:catalog:{}", projection.bot_id);
        Self {
            graph_domain: BOTS_GRAPH_DOMAIN.to_string(),
            event_kind: "bot_catalog".to_string(),
            idempotency_key,
            payload: GraphProjectionPayload::BotCatalog(projection),
        }
    }

    pub fn bot_promotion(projection: BotPromotionProjection) -> Self {
        let idempotency_key = format!("bot:promotion:{}", projection.bot_id);
        Self {
            graph_domain: BOTS_GRAPH_DOMAIN.to_string(),
            event_kind: "bot_promotion".to_string(),
            idempotency_key,
            payload: GraphProjectionPayload::BotPromotion(projection),
        }
    }

    pub fn order_intent(projection: OrderIntentProjection) -> Self {
        let idempotency_key = format!("order:intent:{}", projection.client_order_id);
        Self {
            graph_domain: TRADING_GRAPH_DOMAIN.to_string(),
            event_kind: "order_intent".to_string(),
            idempotency_key,
            payload: GraphProjectionPayload::OrderIntent(projection),
        }
    }

    pub fn submitted_edge(projection: SubmittedEdgeProjection) -> Self {
        let idempotency_key = format!(
            "order:submitted:{}:{}",
            projection.bot_id, projection.client_order_id
        );
        Self {
            graph_domain: TRADING_GRAPH_DOMAIN.to_string(),
            event_kind: "submitted_edge".to_string(),
            idempotency_key,
            payload: GraphProjectionPayload::SubmittedEdge(projection),
        }
    }
}

#[derive(Debug, Error)]
pub enum GraphProjectionOutboxError {
    #[error("outbox store error: {0}")]
    Store(String),
    #[error("neo4j unavailable for drain: {0}")]
    Neo4jUnavailable(String),
    #[error("invalid outbox payload: {0}")]
    InvalidPayload(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrainSummary {
    pub processed: u32,
    pub succeeded: u32,
    pub failed: u32,
}

pub async fn enqueue_graph_projection_outbox_tx(
    tx: &mut Transaction<'_, Postgres>,
    message: &GraphProjectionOutboxMessage,
) -> Result<(), GraphProjectionOutboxError> {
    let payload_json = serde_json::to_value(&message.payload)
        .map_err(|error| GraphProjectionOutboxError::InvalidPayload(error.to_string()))?;
    sqlx::query(
        "INSERT INTO graph_projection_outbox (graph_domain, event_kind, idempotency_key, payload, status)
         VALUES ($1, $2, $3, $4, 'pending')
         ON CONFLICT (graph_domain, idempotency_key) DO UPDATE SET
           event_kind = EXCLUDED.event_kind,
           payload = EXCLUDED.payload,
           status = 'pending',
           processed_at = NULL,
           last_error = NULL",
    )
    .bind(&message.graph_domain)
    .bind(&message.event_kind)
    .bind(&message.idempotency_key)
    .bind(payload_json)
    .execute(&mut **tx)
    .await
    .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
    Ok(())
}

pub async fn enqueue_graph_projection_outbox(
    pool: &PgPool,
    message: &GraphProjectionOutboxMessage,
) -> Result<(), GraphProjectionOutboxError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
    enqueue_graph_projection_outbox_tx(&mut tx, message).await?;
    tx.commit()
        .await
        .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
    Ok(())
}

/// Best-effort inline drain after outbox rows were committed (e.g. same-TX domain persist).
pub async fn graph_projection_drain_best_effort(
    sync: GraphProjectionSync<'_>,
    message_count: usize,
) {
    if message_count == 0 {
        return;
    }
    if let (Some(postgres), Some(neo4j)) = (sync.postgres, sync.neo4j) {
        let drain_limit = (message_count as u32).saturating_add(16);
        if let Err(error) = drain_graph_projection_outbox(postgres.pool(), neo4j, drain_limit).await
        {
            tracing::warn!(
                target: "database",
                %error,
                "graph projection outbox drain failed (rows remain pending/retry)"
            );
        }
    }
}

/// Drains pending/retry rows via Neo4j MERGE. Fail-closed when Neo4j ping fails (no row mutation).
pub async fn drain_graph_projection_outbox(
    pool: &PgPool,
    neo4j: &Neo4jGraph,
    limit: u32,
) -> Result<DrainSummary, GraphProjectionOutboxError> {
    if neo4j.ping().await.is_err() {
        return Err(GraphProjectionOutboxError::Neo4jUnavailable(
            "ping failed before drain".into(),
        ));
    }
    let port = Neo4jCompositeProjection(neo4j);
    drain_graph_projection_outbox_with_port(pool, &port, limit).await
}

pub async fn drain_graph_projection_outbox_with_port(
    pool: &PgPool,
    port: &dyn GraphProjectionPort,
    limit: u32,
) -> Result<DrainSummary, GraphProjectionOutboxError> {
    let cap = limit.clamp(1, 500);
    let mut summary = DrainSummary::default();
    for _ in 0..cap {
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
        let row_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM graph_projection_outbox
             WHERE status IN ('pending', 'retry')
             ORDER BY id
             LIMIT 1
             FOR UPDATE SKIP LOCKED",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;

        let Some(row_id) = row_id else {
            tx.commit()
                .await
                .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
            break;
        };
        let payload_value: serde_json::Value =
            sqlx::query_scalar("SELECT payload FROM graph_projection_outbox WHERE id = $1")
                .bind(row_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;

        sqlx::query("UPDATE graph_projection_outbox SET status = 'processing' WHERE id = $1")
            .bind(row_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
        tx.commit()
            .await
            .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;

        summary.processed += 1;
        let payload: GraphProjectionPayload = serde_json::from_value(payload_value)
            .map_err(|error| GraphProjectionOutboxError::InvalidPayload(error.to_string()))?;
        match apply_payload(port, &payload).await {
            Ok(()) => {
                sqlx::query(
                    "UPDATE graph_projection_outbox
                     SET status = 'done', processed_at = NOW(), last_error = NULL
                     WHERE id = $1",
                )
                .bind(row_id)
                .execute(pool)
                .await
                .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
                summary.succeeded += 1;
            }
            Err(error) => {
                let message = error.to_string();
                sqlx::query(
                    "UPDATE graph_projection_outbox
                     SET status = 'retry',
                         attempt_count = attempt_count + 1,
                         last_error = $2
                     WHERE id = $1",
                )
                .bind(row_id)
                .bind(&message)
                .execute(pool)
                .await
                .map_err(|error| GraphProjectionOutboxError::Store(error.to_string()))?;
                summary.failed += 1;
            }
        }
    }
    Ok(summary)
}

pub async fn graph_projection_best_effort(
    sync: GraphProjectionSync<'_>,
    messages: &[GraphProjectionOutboxMessage],
) {
    if messages.is_empty() {
        return;
    }
    let pool = sync.postgres.map(|db| db.pool());
    if let Some(pool) = pool {
        for message in messages {
            if let Err(error) = enqueue_graph_projection_outbox(pool, message).await {
                tracing::warn!(
                    target: "database",
                    idempotency_key = %message.idempotency_key,
                    %error,
                    "graph projection outbox enqueue failed (PostgreSQL already committed)"
                );
            }
        }
    }
    if let Some(neo4j) = sync.neo4j {
        if let Some(pool) = pool {
            let drain_limit = (messages.len() as u32).saturating_add(16);
            if let Err(error) = drain_graph_projection_outbox(pool, neo4j, drain_limit).await {
                tracing::warn!(
                    target: "database",
                    %error,
                    "graph projection outbox drain failed (rows remain pending/retry)"
                );
            }
        } else if let Err(error) = apply_messages_direct(neo4j, messages).await {
            tracing::warn!(
                target: "database",
                %error,
                "direct neo4j graph projection failed (no PostgreSQL outbox)"
            );
        }
    }
}

async fn apply_messages_direct(
    neo4j: &Neo4jGraph,
    messages: &[GraphProjectionOutboxMessage],
) -> Result<(), GraphProjectionError> {
    let port = Neo4jCompositeProjection(neo4j);
    for message in messages {
        apply_payload(&port, &message.payload).await?;
    }
    Ok(())
}

async fn apply_payload(
    port: &dyn GraphProjectionPort,
    payload: &GraphProjectionPayload,
) -> Result<(), GraphProjectionError> {
    match payload {
        GraphProjectionPayload::AgentHierarchy(p) => port.project_agent_hierarchy(p).await,
        GraphProjectionPayload::BotCatalog(p) => port.project_bot_catalog_entry(p).await,
        GraphProjectionPayload::BotPromotion(p) => port.project_bot_promotion(p).await,
        GraphProjectionPayload::OrderIntent(p) => port.project_order_intent(p).await,
        GraphProjectionPayload::SubmittedEdge(p) => port.project_submitted_edge(p).await,
    }
}

struct Neo4jCompositeProjection<'a>(&'a Neo4jGraph);

#[async_trait::async_trait]
impl GraphProjectionPort for Neo4jCompositeProjection<'_> {
    async fn project_agent_hierarchy(
        &self,
        projection: &AgentHierarchyProjection,
    ) -> Result<(), GraphProjectionError> {
        self.0
            .agent_hierarchy_projector()
            .project_agent_hierarchy(projection)
            .await
    }

    async fn project_bot_catalog_entry(
        &self,
        projection: &BotCatalogProjection,
    ) -> Result<(), GraphProjectionError> {
        self.0
            .bot_projector()
            .project_bot_catalog_entry(projection)
            .await
    }

    async fn project_bot_promotion(
        &self,
        projection: &BotPromotionProjection,
    ) -> Result<(), GraphProjectionError> {
        self.0
            .bot_projector()
            .project_bot_promotion(projection)
            .await
    }

    async fn project_order_intent(
        &self,
        projection: &OrderIntentProjection,
    ) -> Result<(), GraphProjectionError> {
        self.0
            .order_intent_projector()
            .project_order_intent(projection)
            .await
    }

    async fn project_submitted_edge(
        &self,
        projection: &SubmittedEdgeProjection,
    ) -> Result<(), GraphProjectionError> {
        self.0
            .order_intent_projector()
            .project_submitted_edge(projection)
            .await
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::core::database::ProjectedSupervisorKind;
    use std::sync::Mutex;

    struct RecordingPort {
        calls: Mutex<Vec<String>>,
        fail_kind: Mutex<Option<String>>,
    }

    impl RecordingPort {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                fail_kind: Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl GraphProjectionPort for RecordingPort {
        async fn project_agent_hierarchy(
            &self,
            projection: &AgentHierarchyProjection,
        ) -> Result<(), GraphProjectionError> {
            self.calls
                .lock()
                .expect("lock")
                .push(format!("agent:{}", projection.agent_id));
            if self.fail_kind.lock().expect("lock").as_deref() == Some("agent_hierarchy") {
                return Err(GraphProjectionError::Driver("injected".into()));
            }
            Ok(())
        }

        async fn project_bot_catalog_entry(
            &self,
            _: &BotCatalogProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_bot_promotion(
            &self,
            _: &BotPromotionProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_order_intent(
            &self,
            _: &OrderIntentProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_submitted_edge(
            &self,
            _: &SubmittedEdgeProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }
    }

    #[test]
    fn outbox_message_idempotency_keys_are_stable() {
        let msg = GraphProjectionOutboxMessage::agent_hierarchy(AgentHierarchyProjection {
            agency_id: "a1".into(),
            agent_id: "ceo".into(),
            role: "ceo".into(),
            lifecycle: "active".into(),
            supervisor_kind: ProjectedSupervisorKind::Owner,
            supervisor_owner_id: Some("owner".into()),
            supervisor_agent_id: None,
            updated_at_ms: 1,
        });
        assert_eq!(msg.idempotency_key, "agent:a1:ceo");
        assert_eq!(msg.graph_domain, AGENTS_GRAPH_DOMAIN);
    }

    #[tokio::test]
    async fn pg_graph_projection_outbox_enqueue_and_drain_mock() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let pool = db.pool();
        let key = format!(
            "test-agent-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let projection = AgentHierarchyProjection {
            agency_id: "agency-test".into(),
            agent_id: key.clone(),
            role: "worker".into(),
            lifecycle: "active".into(),
            supervisor_kind: ProjectedSupervisorKind::Agent,
            supervisor_owner_id: None,
            supervisor_agent_id: Some("ceo".into()),
            updated_at_ms: 42,
        };
        let message = GraphProjectionOutboxMessage::agent_hierarchy(projection);
        enqueue_graph_projection_outbox(pool, &message)
            .await
            .expect("enqueue");

        let port = RecordingPort::new();
        let summary = drain_graph_projection_outbox_with_port(pool, &port, 4)
            .await
            .expect("drain");
        assert_eq!(summary.succeeded, 1);
        assert!(port
            .calls
            .lock()
            .expect("lock")
            .iter()
            .any(|c| c.contains(&key)));

        let status: String = sqlx::query_scalar(
            "SELECT status FROM graph_projection_outbox WHERE idempotency_key = $1",
        )
        .bind(format!("agent:agency-test:{}", key))
        .fetch_one(pool)
        .await
        .expect("status");
        assert_eq!(status, "done");
    }

    #[tokio::test]
    async fn pg_graph_projection_outbox_drain_marks_retry_on_port_failure() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let pool = db.pool();
        let key = format!(
            "fail-agent-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let message = GraphProjectionOutboxMessage::agent_hierarchy(AgentHierarchyProjection {
            agency_id: "agency-fail".into(),
            agent_id: key.clone(),
            role: "worker".into(),
            lifecycle: "active".into(),
            supervisor_kind: ProjectedSupervisorKind::Agent,
            supervisor_owner_id: None,
            supervisor_agent_id: Some("ceo".into()),
            updated_at_ms: 1,
        });
        enqueue_graph_projection_outbox(pool, &message)
            .await
            .expect("enqueue");
        let port = RecordingPort::new();
        *port.fail_kind.lock().expect("lock") = Some("agent_hierarchy".into());
        let summary = drain_graph_projection_outbox_with_port(pool, &port, 1)
            .await
            .expect("drain");
        assert_eq!(summary.failed, 1);
        let status: String = sqlx::query_scalar(
            "SELECT status FROM graph_projection_outbox WHERE idempotency_key = $1",
        )
        .bind(format!("agent:agency-fail:{}", key))
        .fetch_one(pool)
        .await
        .expect("status");
        assert_eq!(status, "retry");
    }
}
