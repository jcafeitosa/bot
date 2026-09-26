use std::{env, time::Duration};

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
        let options: PgConnectOptions = raw
            .parse()
            .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
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

    pub async fn migrate(&self) -> Result<(), PersistenceError> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/persistence/migrations");
        let migrator = sqlx::migrate::Migrator::new(path.as_path()).await?;
        migrator.run(self.pool()).await?;
        Ok(())
    }

    pub async fn persist_dataset(
        &self,
        dataset: &crate::market::HistoricalDataset,
    ) -> Result<(), PersistenceError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO market_datasets (dataset_id, symbol, base_timeframe, start_ms, end_ms, candle_count, gap_count, source) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (dataset_id) DO NOTHING"
        ).bind(&dataset.manifest.dataset_id).bind(&dataset.manifest.symbol).bind(&dataset.manifest.base_timeframe)
         .bind(dataset.manifest.start_ms).bind(dataset.manifest.end_ms).bind(dataset.manifest.candle_count as i64)
         .bind(dataset.manifest.gap_count as i64).bind(&dataset.manifest.source).execute(&mut *tx).await?;
        for candle in &dataset.candles {
            sqlx::query(
                "INSERT INTO candles_1m (symbol, time_ms, open, high, low, close, volume, dataset_id) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (symbol, time_ms) DO NOTHING"
            ).bind(&dataset.manifest.symbol).bind(candle.timestamp_ms).bind(candle.open).bind(candle.high)
             .bind(candle.low).bind(candle.close).bind(candle.volume).bind(&dataset.manifest.dataset_id)
             .execute(&mut *tx).await?;
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
}

pub fn persist_market_data_enabled() -> bool {
    env::var("PERSIST_MARKET_DATA")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::market::{Candle, HistoricalDataset};

    fn fixture_dataset() -> HistoricalDataset {
        let candles = (0..5)
            .map(|i| {
                let close = 100.0 + i as f64;
                Candle {
                    timestamp_ms: i as i64 * 60_000,
                    open: close,
                    high: close + 1.0,
                    low: close - 1.0,
                    close,
                    volume: 1.0,
                }
            })
            .collect();
        HistoricalDataset::from_1m("BTC/USDT", "integration-test", candles).unwrap()
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at PostgreSQL database trading_bot"]
    async fn persist_dataset_round_trip() {
        let db = Database::connect_from_env()
            .await
            .expect("DATABASE_URL must be set for ignored integration test");
        db.migrate().await.expect("migrations");
        let dataset = fixture_dataset();
        let dataset_id = dataset.manifest.dataset_id.clone();
        let expected = dataset.manifest.candle_count as i64;

        db.persist_dataset(&dataset)
            .await
            .expect("persist_dataset should succeed");
        let count = db
            .candle_count_for_dataset(&dataset_id)
            .await
            .expect("count query");
        assert_eq!(count, expected);

        db.persist_dataset(&dataset)
            .await
            .expect("second persist should be idempotent");
        let count_again = db
            .candle_count_for_dataset(&dataset_id)
            .await
            .expect("count query after replay");
        assert_eq!(count_again, expected);
    }
}
