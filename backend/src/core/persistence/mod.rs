use std::{env, time::Duration};

#[allow(unused_imports)]
pub use dataset::{CandleRow, MarketDatasetManifestRow, MarketDatasetPersistInput};

mod dataset;

use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("DATABASE_URL is required to enable PostgreSQL persistence")]
    MissingUrl,
    #[error("DATABASE_URL must point to the dedicated trading_bot database")]
    WrongDatabase,
    #[error("DATABASE_URL is invalid")]
    InvalidUrl,
    #[error("database connection failed: {0}")]
    Connect(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect_from_env() -> Result<Self, PersistenceError> {
        let raw = env::var("DATABASE_URL").map_err(|_| PersistenceError::MissingUrl)?;
        Self::connect_from_url(&raw).await
    }

    pub async fn connect_from_url(raw: &str) -> Result<Self, PersistenceError> {
        let options: PgConnectOptions = raw.parse().map_err(|_| PersistenceError::InvalidUrl)?;
        if options.get_database() != Some("trading_bot") {
            return Err(PersistenceError::WrongDatabase);
        }
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await?;
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&pool)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<(), PersistenceError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(self.pool())
            .await?;
        Ok(())
    }

    pub async fn migrate(&self) -> Result<(), PersistenceError> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/core/persistence/migrations");
        let migrator = sqlx::migrate::Migrator::new(path.as_path()).await?;
        migrator.run(self.pool()).await?;
        Ok(())
    }

    pub async fn persist_dataset(
        &self,
        dataset: &MarketDatasetPersistInput,
    ) -> Result<(), PersistenceError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO market_datasets (dataset_id, symbol, base_timeframe, start_ms, end_ms, candle_count, gap_count, source) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (dataset_id) DO NOTHING"
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
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (symbol, time_ms) DO NOTHING"
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
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::bigint FROM candles_1m WHERE dataset_id = $1",
        )
        .bind(dataset_id)
        .fetch_one(self.pool())
        .await?;
        Ok(count)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn table_exists(&self, table: &str) -> Result<bool, PersistenceError> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = $1
            )",
        )
        .bind(table)
        .fetch_one(self.pool())
        .await?;
        Ok(exists)
    }
}

#[cfg(test)]
mod migration_scaffold_tests {
    const SCAFFOLD_SQL: &str = include_str!("migrations/0002_agents_bots_scaffold.sql");

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

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at PostgreSQL database trading_bot"]
    async fn postgres_scaffold_tables_exist_after_migrate() {
        let db = super::Database::connect_from_env()
            .await
            .expect("DATABASE_URL must be set for ignored integration test");
        db.migrate().await.expect("migrations");
        for table in [
            "agent_identities",
            "agent_identity_events",
            "bot_catalog_entries",
        ] {
            assert!(
                db.table_exists(table).await.expect("table_exists query"),
                "missing table {table} after migrate"
            );
        }
    }
}
