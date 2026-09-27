use sqlx::PgPool;

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
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("idempotency lookup failed: {error}"))
        })?;
        Ok(exists)
    }

    pub async fn record_completed(&self, key: &str) -> Result<(), OrdersError> {
        sqlx::query(
            "INSERT INTO order_idempotency_keys (client_order_id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(key)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("idempotency record failed: {error}"))
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
}
