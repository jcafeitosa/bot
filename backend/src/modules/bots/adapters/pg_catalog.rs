use sqlx::PgPool;

use super::persistence::BotCatalogStore;
use crate::core::config::OperationMode;
use crate::core::database::PostgresDatabase;
use crate::modules::bots::models::{BotDefinition, BotId, StrategyId, StrategyVersion};

/// Gate 1: durable bot catalog rows in `bot_catalog_entries` (fail-closed; no auth).
#[derive(Clone)]
pub struct PgBotCatalogStore {
    pool: PgPool,
}

impl PgBotCatalogStore {
    pub fn new(db: &PostgresDatabase) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }
}

#[async_trait::async_trait]
impl BotCatalogStore for PgBotCatalogStore {
    async fn save_catalog(&mut self, entries: &[BotDefinition]) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM bot_catalog_entries")
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
        for entry in entries {
            sqlx::query(
                "INSERT INTO bot_catalog_entries \
                 (bot_id, strategy_id, strategy_version, timeframe, symbol, operation_mode) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(entry.id.as_str())
            .bind(entry.strategy_id.as_str())
            .bind(entry.strategy_version.0 as i32)
            .bind(&entry.timeframe)
            .bind(&entry.symbol)
            .bind(operation_mode_to_sql(entry.operation))
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
        }
        tx.commit().await.map_err(|error| error.to_string())
    }

    async fn load_catalog(&self) -> Result<Vec<BotDefinition>, String> {
        let rows = sqlx::query_as::<_, BotCatalogRow>(
            "SELECT bot_id, strategy_id, strategy_version, timeframe, symbol, operation_mode \
             FROM bot_catalog_entries ORDER BY bot_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        rows.into_iter()
            .map(BotCatalogRow::into_definition)
            .collect()
    }
}

#[derive(sqlx::FromRow)]
struct BotCatalogRow {
    bot_id: String,
    strategy_id: String,
    strategy_version: i32,
    timeframe: String,
    symbol: String,
    operation_mode: String,
}

impl BotCatalogRow {
    fn into_definition(self) -> Result<BotDefinition, String> {
        let strategy_id = StrategyId::new(self.strategy_id).map_err(|error| error.to_string())?;
        if self.strategy_version < 0 {
            return Err("negative strategy_version in database".into());
        }
        let operation = operation_mode_from_sql(&self.operation_mode)?;
        let id = BotId::new(
            &strategy_id,
            StrategyVersion(self.strategy_version as u32),
            &self.timeframe,
            &self.symbol,
        )
        .map_err(|error| error.to_string())?;
        if id.as_str() != self.bot_id {
            return Err("bot_id column does not match strategy/timeframe/symbol".into());
        }
        Ok(BotDefinition {
            id,
            strategy_id,
            strategy_version: StrategyVersion(self.strategy_version as u32),
            timeframe: self.timeframe,
            symbol: self.symbol,
            operation,
        })
    }
}

pub fn operation_mode_to_sql(mode: OperationMode) -> &'static str {
    match mode {
        OperationMode::Hft => "hft",
        OperationMode::Scalper => "scalper",
        OperationMode::DayTrader => "day_trader",
        OperationMode::SwingTrader => "swing_trader",
    }
}

pub fn operation_mode_from_sql(raw: &str) -> Result<OperationMode, String> {
    match raw {
        "hft" => Ok(OperationMode::Hft),
        "scalper" => Ok(OperationMode::Scalper),
        "day_trader" => Ok(OperationMode::DayTrader),
        "swing_trader" => Ok(OperationMode::SwingTrader),
        other => Err(format!("unknown operation_mode in database: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_mode_sql_round_trip() {
        for mode in [
            OperationMode::Hft,
            OperationMode::Scalper,
            OperationMode::DayTrader,
            OperationMode::SwingTrader,
        ] {
            let sql = operation_mode_to_sql(mode);
            assert_eq!(operation_mode_from_sql(sql).unwrap(), mode);
        }
    }

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at PostgreSQL 18+ database trading_bot with migrations applied"]
    async fn pg_catalog_store_round_trip() {
        use crate::core::persistence::Database;
        use crate::modules::backtest::models::StrategyDefinition;
        use crate::modules::bots::models::MonitorEvaluatorKind;
        use crate::modules::bots::{build_catalog_from_config, persist_catalog_snapshot};

        let db = Database::connect_from_env().await.expect("DATABASE_URL");
        db.migrate().await.expect("migrate");
        let mut store = PgBotCatalogStore::new(db.as_postgres());
        let strategy = StrategyDefinition {
            id: StrategyId::new("sma-cross").unwrap(),
            version: StrategyVersion(1),
            name: "SMA".into(),
            fast_period: 5,
            slow_period: 20,
            evaluator: MonitorEvaluatorKind::default(),
        };
        let mut config = crate::core::config::Config {
            operation: OperationMode::DayTrader,
            ..Default::default()
        };
        config.market.timeframe = "15m".into();
        config.validate().unwrap();
        let built = persist_catalog_snapshot(&config, &strategy, &mut store)
            .await
            .expect("persist snapshot");
        let loaded = store.load_catalog().await.expect("load");
        assert_eq!(built.len(), loaded.len());
        for entry in &built {
            let round_tripped = loaded
                .iter()
                .find(|row| row.id == entry.id)
                .expect("load_catalog missing bot_id present in snapshot");
            assert_eq!(entry.strategy_id, round_tripped.strategy_id);
            assert_eq!(entry.strategy_version, round_tripped.strategy_version);
            assert_eq!(entry.timeframe, round_tripped.timeframe);
            assert_eq!(entry.symbol, round_tripped.symbol);
            assert_eq!(entry.operation, round_tripped.operation);
        }
    }
}
