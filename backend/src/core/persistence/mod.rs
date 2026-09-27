#[allow(unused_imports)]
pub use dataset::{CandleRow, MarketDatasetManifestRow, MarketDatasetPersistInput};

mod dataset;

#[cfg(test)]
pub mod pg_integration;

use crate::core::database::{DatabaseError, PostgresDatabase};

pub type PersistenceError = DatabaseError;

/// Domain-facing PostgreSQL handle (pool + market dataset writes).
#[derive(Clone)]
pub struct Database(PostgresDatabase);

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database")
    }
}

impl Database {
    pub async fn connect_from_env() -> Result<Self, PersistenceError> {
        PostgresDatabase::connect_from_env().await.map(Self)
    }

    pub async fn connect_from_url(raw: &str) -> Result<Self, PersistenceError> {
        PostgresDatabase::connect_from_url(raw).await.map(Self)
    }

    pub fn as_postgres(&self) -> &PostgresDatabase {
        &self.0
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        self.0.pool()
    }

    #[allow(dead_code)]
    pub async fn ping(&self) -> Result<(), PersistenceError> {
        self.0.ping().await
    }

    pub async fn migrate(&self) -> Result<(), PersistenceError> {
        self.0.migrate().await
    }

    pub async fn persist_dataset(
        &self,
        dataset: &MarketDatasetPersistInput,
    ) -> Result<(), PersistenceError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query(
            "INSERT INTO market_datasets (dataset_id, symbol, base_timeframe, start_ms, end_ms, candle_count, gap_count, source) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (dataset_id) DO NOTHING",
        )
        .bind(&dataset.manifest.dataset_id)
        .bind(&dataset.manifest.symbol)
        .bind(&dataset.manifest.base_timeframe)
        .bind(dataset.manifest.start_ms)
        .bind(dataset.manifest.end_ms)
        .bind(dataset.manifest.candle_count as i64)
        .bind(dataset.manifest.gap_count as i64)
        .bind(&dataset.manifest.source)
        .execute(&mut *tx)
        .await?;
        for candle in &dataset.candles {
            sqlx::query(
                "INSERT INTO candles_1m (symbol, time_ms, open, high, low, close, volume, dataset_id) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (symbol, time_ms) DO NOTHING",
            )
            .bind(&dataset.manifest.symbol)
            .bind(candle.timestamp_ms)
            .bind(candle.open)
            .bind(candle.high)
            .bind(candle.low)
            .bind(candle.close)
            .bind(candle.volume)
            .bind(&dataset.manifest.dataset_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn candle_count_for_dataset(
        &self,
        dataset_id: &str,
    ) -> Result<i64, PersistenceError> {
        self.0.candle_count_for_dataset(dataset_id).await
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn table_exists(&self, table: &str) -> Result<bool, PersistenceError> {
        self.0.table_exists(table).await
    }
}

#[cfg(test)]
mod migration_scaffold_tests {
    const SCAFFOLD_SQL: &str = include_str!("../database/migrations/0002_agents_bots_scaffold.sql");

    #[test]
    fn migration_scaffold_sql_declares_core_tables() {
        for table in [
            "agent_identities",
            "agent_identity_events",
            "bot_catalog_entries",
        ] {
            assert!(
                SCAFFOLD_SQL.contains(table),
                "0002 scaffold must define table {table}"
            );
        }
    }

    const ORDER_IDEMPOTENCY_SQL: &str =
        include_str!("../database/migrations/0004_order_idempotency_keys.sql");

    #[test]
    fn migration_order_idempotency_sql_declares_table() {
        assert!(
            ORDER_IDEMPOTENCY_SQL.contains("order_idempotency_keys"),
            "0004 must define order_idempotency_keys"
        );
    }

    const AGENT_PROMOTE_RUNTIME_SQL: &str =
        include_str!("../database/migrations/0005_agent_promote_runtime_bot.sql");

    #[test]
    fn migration_agent_promote_runtime_sql_adds_column() {
        assert!(
            AGENT_PROMOTE_RUNTIME_SQL.contains("promote_runtime_bot"),
            "0005 must add promote_runtime_bot to agent_identities"
        );
    }

    const ORDER_RECONCILIATION_SQL: &str =
        include_str!("../database/migrations/0006_order_reconciliation.sql");

    #[test]
    fn migration_order_reconciliation_sql_declares_table() {
        assert!(
            ORDER_RECONCILIATION_SQL.contains("order_reconciliation"),
            "0006 must define order_reconciliation"
        );
    }

    #[tokio::test]
    async fn postgres_scaffold_tables_exist_after_migrate() {
        let Some(db) = super::pg_integration::database_for_integration_test().await else {
            return;
        };
        for table in [
            "agent_identities",
            "agent_identity_events",
            "bot_catalog_entries",
            "order_idempotency_keys",
            "order_reconciliation",
            "graph_projection_outbox",
        ] {
            assert!(
                db.table_exists(table).await.expect("table_exists query"),
                "missing table {table} after migrate"
            );
        }
        let promote_col = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'agent_identities'
                  AND column_name = 'promote_runtime_bot'
            )",
        )
        .fetch_one(db.pool())
        .await
        .expect("column_exists query");
        assert!(
            promote_col,
            "missing agent_identities.promote_runtime_bot after migration 0005"
        );
    }
}
