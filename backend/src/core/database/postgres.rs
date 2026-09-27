use std::time::Duration;

use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use thiserror::Error;

pub const REQUIRED_PG_EXTENSIONS: &[&str] = &["timescaledb", "vector"];
/// PostgreSQL 18.0 (`server_version_num` 180000).
pub const MIN_SERVER_VERSION_NUM: i32 = 180_000;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("DATABASE_URL is required to enable PostgreSQL persistence")]
    MissingUrl,
    #[error("DATABASE_URL must point to the dedicated trading_bot database")]
    WrongDatabase,
    #[error("DATABASE_URL is invalid")]
    InvalidUrl,
    #[error("PostgreSQL server version must be 18 or newer")]
    UnsupportedVersion,
    #[error("database connection failed: {0}")]
    Connect(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionHealth {
    pub ok: bool,
    pub present: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl std::fmt::Debug for PostgresDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PostgresDatabase")
    }
}

impl PostgresDatabase {
    pub async fn connect_from_env() -> Result<Self, DatabaseError> {
        let raw = crate::core::config::postgres_url_from_env()
            .map_err(|_| DatabaseError::InvalidUrl)?
            .ok_or(DatabaseError::MissingUrl)?;
        Self::connect_from_url(&raw).await
    }

    pub async fn connect_from_url(raw: &str) -> Result<Self, DatabaseError> {
        let options: PgConnectOptions = raw.parse().map_err(|_| DatabaseError::InvalidUrl)?;
        if options.get_database() != Some("trading_bot") {
            return Err(DatabaseError::WrongDatabase);
        }
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await?;
        Self::assert_server_version(&pool).await?;
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&pool)
            .await?;
        Ok(Self { pool })
    }

    async fn assert_server_version(pool: &PgPool) -> Result<(), DatabaseError> {
        let version_num: i32 =
            sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
                .fetch_one(pool)
                .await?;
        if version_num < MIN_SERVER_VERSION_NUM {
            return Err(DatabaseError::UnsupportedVersion);
        }
        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<(), DatabaseError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(self.pool())
            .await?;
        Ok(())
    }

    pub async fn migrate(&self) -> Result<(), DatabaseError> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core/database/migrations");
        let migrator = sqlx::migrate::Migrator::new(path.as_path()).await?;
        migrator.run(self.pool()).await?;
        if let Err(error) = crate::core::providers::credentials::reload_from_pool(self.pool()).await
        {
            tracing::warn!(target: "providers", %error, "provider credential cache reload failed");
        }
        Ok(())
    }

    pub async fn installed_extensions(&self) -> Result<Vec<String>, DatabaseError> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT extname FROM pg_extension WHERE extname = ANY($1::text[])",
        )
        .bind(REQUIRED_PG_EXTENSIONS)
        .fetch_all(self.pool())
        .await?;
        Ok(rows)
    }

    pub async fn extension_health(&self) -> Result<ExtensionHealth, DatabaseError> {
        let present = self.installed_extensions().await?;
        let missing = REQUIRED_PG_EXTENSIONS
            .iter()
            .filter(|name| !present.iter().any(|p| p == *name))
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>();
        Ok(ExtensionHealth {
            ok: missing.is_empty(),
            present,
            missing,
        })
    }

    pub async fn table_exists(&self, table: &str) -> Result<bool, DatabaseError> {
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn candle_count_for_dataset(&self, dataset_id: &str) -> Result<i64, DatabaseError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::bigint FROM candles_1m WHERE dataset_id = $1",
        )
        .bind(dataset_id)
        .fetch_one(self.pool())
        .await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_extensions_are_documented() {
        assert!(REQUIRED_PG_EXTENSIONS.contains(&"timescaledb"));
        assert!(REQUIRED_PG_EXTENSIONS.contains(&"vector"));
    }

    #[tokio::test]
    async fn connect_rejects_wrong_database_without_network() {
        let err = match PostgresDatabase::connect_from_url("postgresql://localhost/other").await {
            Err(error) => error,
            Ok(_) => panic!("expected WrongDatabase"),
        };
        assert!(matches!(err, DatabaseError::WrongDatabase));
    }
}
