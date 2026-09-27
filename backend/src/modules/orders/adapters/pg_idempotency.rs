use sqlx::PgPool;

use super::pg_store_error::orders_pg_store_error;
use crate::core::database::{
    enqueue_graph_projection_outbox_tx, GraphProjectionOutboxError, GraphProjectionOutboxMessage,
    PostgresDatabase,
};
use crate::modules::orders::OrdersError;

/// Durable idempotency keys when PostgreSQL is available (`order_idempotency_keys`).
#[derive(Clone)]
pub struct PgOrderIdempotencyStore {
    pool: PgPool,
}

impl PgOrderIdempotencyStore {
    pub fn new(db: &PostgresDatabase) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn is_completed(&self, key: &str) -> Result<bool, OrdersError> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM order_idempotency_keys WHERE client_order_id = $1)",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| orders_pg_store_error("idempotency lookup", error))?;
        Ok(exists)
    }

    pub async fn record_completed(&self, key: &str) -> Result<(), OrdersError> {
        sqlx::query(
            "INSERT INTO order_idempotency_keys (client_order_id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(key)
        .execute(&self.pool)
        .await
        .map_err(|error| orders_pg_store_error("idempotency record", error))?;
        Ok(())
    }

    /// Inserts the key before order execution. Returns `true` when this call claimed the key.
    pub async fn try_claim(&self, key: &str) -> Result<bool, OrdersError> {
        let inserted = sqlx::query_scalar::<_, Option<String>>(
            "INSERT INTO order_idempotency_keys (client_order_id) VALUES ($1)
             ON CONFLICT (client_order_id) DO NOTHING
             RETURNING client_order_id",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| orders_pg_store_error("idempotency claim", error))?;
        Ok(inserted.is_some())
    }

    /// Drops a claim when execution failed so the client may retry with the same key.
    pub async fn release_claim(&self, key: &str) -> Result<(), OrdersError> {
        sqlx::query("DELETE FROM order_idempotency_keys WHERE client_order_id = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|error| orders_pg_store_error("idempotency release", error))?;
        Ok(())
    }

    /// Atomically persists idempotency (noop if row already claimed) and enqueues graph outbox rows (F2.1.3+ orders).
    pub async fn persist_idempotency_and_enqueue_graph_projection(
        &self,
        key: &str,
        messages: &[GraphProjectionOutboxMessage],
    ) -> Result<(), OrdersError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| orders_pg_store_error("idempotency and graph outbox tx", error))?;
        sqlx::query(
            "INSERT INTO order_idempotency_keys (client_order_id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(|error| orders_pg_store_error("idempotency and graph outbox tx", error))?;
        for message in messages {
            enqueue_graph_projection_outbox_tx(&mut tx, message)
                .await
                .map_err(|error| match error {
                    GraphProjectionOutboxError::Store(message) => OrdersError::StoreUnavailable(
                        format!("graph projection outbox enqueue: {message}"),
                    ),
                    GraphProjectionOutboxError::InvalidPayload(message) => {
                        OrdersError::StoreUnavailable(format!(
                            "graph projection outbox payload: {message}"
                        ))
                    }
                    GraphProjectionOutboxError::Neo4jUnavailable(_) => {
                        OrdersError::StoreUnavailable(
                            "graph projection outbox enqueue: neo4j unavailable".into(),
                        )
                    }
                })?;
        }
        tx.commit().await.map_err(|error| {
            orders_pg_store_error("idempotency and graph outbox tx commit", error)
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pg_order_idempotency_round_trip() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let store = PgOrderIdempotencyStore::new(db.as_postgres());
        let key = format!(
            "idem-pg-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        assert!(!store.is_completed(&key).await.expect("lookup"));
        store.record_completed(&key).await.expect("record");
        assert!(store.is_completed(&key).await.expect("lookup again"));
    }

    #[tokio::test]
    async fn pg_order_idempotency_try_claim_and_release() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let store = PgOrderIdempotencyStore::new(db.as_postgres());
        let key = format!(
            "idem-claim-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        assert!(store.try_claim(&key).await.expect("first claim"));
        assert!(!store.try_claim(&key).await.expect("duplicate claim"));
        store.release_claim(&key).await.expect("release");
        assert!(!store
            .is_completed(&key)
            .await
            .expect("lookup after release"));
        assert!(store.try_claim(&key).await.expect("re-claim"));
    }

    #[tokio::test]
    async fn pg_order_idempotency_store_unavailable_when_table_missing() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let pool = db.as_postgres().pool().clone();
        const HIDDEN: &str = "order_idempotency_keys_pg_test_hidden";
        sqlx::query(&format!(
            "ALTER TABLE order_idempotency_keys RENAME TO {HIDDEN}"
        ))
        .execute(&pool)
        .await
        .expect("hide table for test");
        let store = PgOrderIdempotencyStore::new(db.as_postgres());
        let err = store.is_completed("probe-key").await.unwrap_err();
        assert!(matches!(err, OrdersError::StoreUnavailable(_)));
        sqlx::query(&format!(
            "ALTER TABLE {HIDDEN} RENAME TO order_idempotency_keys"
        ))
        .execute(&pool)
        .await
        .expect("restore table");
    }

    #[tokio::test]
    async fn pg_order_idempotency_and_graph_projection_same_transaction() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        use crate::modules::orders::adapters::graph_projection::{
            order_graph_projection_outbox_messages, RedactedOrderSubmitSnapshot,
        };
        use crate::modules::orders::OrderSide;

        let store = PgOrderIdempotencyStore::new(db.as_postgres());
        let key = format!(
            "idem-outbox-tx-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let snapshot = RedactedOrderSubmitSnapshot {
            client_order_id: key.clone(),
            symbol: "BTC/USDT".into(),
            side: OrderSide::Buy,
            execution_mode: "paper".into(),
            submitting_bot_id: None,
        };
        let messages = order_graph_projection_outbox_messages(&snapshot);
        store
            .persist_idempotency_and_enqueue_graph_projection(&key, &messages)
            .await
            .expect("persist tx");
        assert!(store.is_completed(&key).await.expect("idem"));
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM graph_projection_outbox WHERE idempotency_key = $1 AND status = 'pending'",
        )
        .bind(format!("order:intent:{key}"))
        .fetch_one(db.pool())
        .await
        .expect("outbox count");
        assert_eq!(pending, 1);
    }
}
