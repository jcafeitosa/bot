use sqlx::PgPool;

use super::persistence::BotCatalogStore;
use crate::core::config::OperationMode;
use crate::core::database::{
    enqueue_graph_projection_outbox_tx, GraphProjectionOutboxError, PostgresDatabase,
};
use crate::modules::bots::adapters::graph_projection::{
    bot_catalog_graph_projection_messages, bot_demotion_graph_projection_message,
    bot_promotion_graph_projection_message,
};
use crate::modules::bots::models::{
    BotDefinition, BotId, BotPromotionRecord, StrategyId, StrategyVersion,
};

fn graph_outbox_enqueue_error(error: GraphProjectionOutboxError) -> String {
    match error {
        GraphProjectionOutboxError::Store(message) => {
            format!("graph projection outbox enqueue: {message}")
        }
        GraphProjectionOutboxError::InvalidPayload(message) => {
            format!("graph projection outbox payload: {message}")
        }
        GraphProjectionOutboxError::Neo4jUnavailable(_) => {
            "graph projection outbox enqueue: neo4j unavailable".into()
        }
    }
}

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

    /// Enqueues bot promotion graph outbox in one PG transaction (runtime promotion has no PG SoT).
    pub async fn enqueue_bot_promotion_graph_projection(
        &self,
        record: &BotPromotionRecord,
        agency_id: Option<&str>,
    ) -> Result<(), String> {
        let message = bot_promotion_graph_projection_message(record, agency_id);
        let mut tx = self.pool.begin().await.map_err(|error| error.to_string())?;
        enqueue_graph_projection_outbox_tx(&mut tx, &message)
            .await
            .map_err(graph_outbox_enqueue_error)?;
        tx.commit().await.map_err(|error| error.to_string())
    }

    /// Enqueues bot demotion graph outbox in one PG transaction.
    pub async fn enqueue_bot_demotion_graph_projection(&self, bot_id: &str) -> Result<(), String> {
        let message = bot_demotion_graph_projection_message(bot_id);
        let mut tx = self.pool.begin().await.map_err(|error| error.to_string())?;
        enqueue_graph_projection_outbox_tx(&mut tx, &message)
            .await
            .map_err(graph_outbox_enqueue_error)?;
        tx.commit().await.map_err(|error| error.to_string())
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
        for message in bot_catalog_graph_projection_messages(entries) {
            enqueue_graph_projection_outbox_tx(&mut tx, &message)
                .await
                .map_err(graph_outbox_enqueue_error)?;
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
    async fn pg_catalog_store_round_trip() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        use crate::modules::backtest::models::StrategyDefinition;
        use crate::modules::bots::models::MonitorEvaluatorKind;
        use crate::modules::bots::{build_catalog_from_config, persist_catalog_snapshot};

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
    #[tokio::test]
    async fn pg_bot_catalog_and_graph_projection_same_transaction() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        use crate::modules::backtest::models::StrategyDefinition;
        use crate::modules::bots::models::MonitorEvaluatorKind;
        use crate::modules::bots::persist_catalog_snapshot;

        let mut store = PgBotCatalogStore::new(db.as_postgres());
        let strategy = StrategyDefinition {
            id: StrategyId::new("sma-cross-outbox").unwrap(),
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
        assert!(!built.is_empty());
        let sample = &built[0];
        let message = crate::modules::bots::adapters::graph_projection::bot_catalog_graph_projection_messages(&built)[0]
            .idempotency_key
            .clone();
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM graph_projection_outbox WHERE idempotency_key = $1 AND status = 'pending'",
        )
        .bind(&message)
        .fetch_one(db.pool())
        .await
        .expect("outbox count");
        assert_eq!(pending, 1);
        let rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM bot_catalog_entries WHERE bot_id = $1")
                .bind(sample.id.as_str())
                .fetch_one(db.pool())
                .await
                .expect("catalog row");
        assert_eq!(rows, 1);
    }
}
