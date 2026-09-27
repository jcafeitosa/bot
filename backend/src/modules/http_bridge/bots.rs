use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::config::Config;
use crate::core::error::BotError;
use crate::modules::backtest::models::StrategyDefinition;
use crate::modules::bots::{
    build_catalog_from_config, full_ranking, persist_catalog_snapshot, BotCatalogStore,
    BotDefinition, BotMetrics, BotRankingReport, StrategyId, StrategyVersion,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct BotCatalogEntry {
    pub bot_id: String,
    pub strategy_id: String,
    pub strategy_version: u32,
    pub timeframe: String,
    pub symbol: String,
    pub operation: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BotCatalogResponse {
    pub bots: Vec<BotCatalogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct BotRankingRequest {
    pub metrics: Vec<BotMetrics>,
}

#[derive(Debug, Serialize)]
pub struct BotRankingResponse {
    pub report: BotRankingReport,
}

fn strategy_from_config(config: &Config) -> Result<StrategyDefinition, BotError> {
    Ok(StrategyDefinition {
        id: StrategyId::new("sma-cross").map_err(|e| BotError::Configuration(e.to_string()))?,
        version: StrategyVersion(1),
        name: "SMA crossover".into(),
        fast_period: config.strategy.sma_fast,
        slow_period: config.strategy.sma_slow,
    })
}

fn map_bot(def: BotDefinition) -> BotCatalogEntry {
    BotCatalogEntry {
        bot_id: def.id.to_string(),
        strategy_id: def.strategy_id.to_string(),
        strategy_version: def.strategy_version.0,
        timeframe: def.timeframe,
        symbol: def.symbol,
        operation: format!("{:?}", def.operation),
    }
}

pub fn catalog_for_config(config: &Config) -> Result<BotCatalogResponse, BotError> {
    let strategy = strategy_from_config(config)?;
    let bots = build_catalog_from_config(config, &strategy)
        .map_err(|e| BotError::Configuration(e.to_string()))?
        .into_iter()
        .map(map_bot)
        .collect();
    Ok(BotCatalogResponse { bots })
}

pub fn ranking_from_metrics(metrics: Vec<BotMetrics>) -> Result<BotRankingResponse, BotError> {
    let report = full_ranking(metrics).map_err(|e| BotError::Configuration(e.to_string()))?;
    Ok(BotRankingResponse { report })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BotCatalogPersistResponse {
    pub bots: Vec<BotCatalogEntry>,
    pub persisted: bool,
}

pub async fn persist_catalog_for_config(
    config: &Config,
    store: &mut impl BotCatalogStore,
) -> Result<BotCatalogPersistResponse, BotError> {
    let strategy = strategy_from_config(config)?;
    let defs = persist_catalog_snapshot(config, &strategy, store)
        .await
        .map_err(|e| BotError::Configuration(e.to_string()))?;
    Ok(BotCatalogPersistResponse {
        bots: defs.into_iter().map(map_bot).collect(),
        persisted: true,
    })
}

pub async fn catalog_from_store(
    store: &impl BotCatalogStore,
) -> Result<BotCatalogResponse, BotError> {
    let defs = store
        .load_catalog()
        .await
        .map_err(BotError::Configuration)?;
    Ok(BotCatalogResponse {
        bots: defs.into_iter().map(map_bot).collect(),
    })
}
