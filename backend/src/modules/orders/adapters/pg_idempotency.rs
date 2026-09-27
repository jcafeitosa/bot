use sqlx::PgPool;

use super::pg_store_error::orders_pg_store_error;
use crate::core::database::PostgresDatabase;
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
}
