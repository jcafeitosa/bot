#![allow(dead_code)]

use crate::core::config::{Config, OperationMode};
use crate::modules::bots::adapters::BotCatalogStore;
use crate::modules::bots::models::{BotDefinition, BotsError, StrategySpec};

pub fn enumerate_timeframes_for_mode(mode: OperationMode) -> &'static [&'static str] {
    mode.supported_timeframes()
}

pub fn build_bot_definition(
    strategy: &impl StrategySpec,
    timeframe: &str,
    symbol: &str,
    operation: OperationMode,
) -> Result<BotDefinition, BotsError> {
    let definition = BotDefinition::new(strategy, timeframe, symbol, operation)?;
    let _ = definition.identity()?.bot_id()?;
    Ok(definition)
}

pub fn build_catalog_from_config(
    config: &Config,
    strategy: &impl StrategySpec,
) -> Result<Vec<BotDefinition>, BotsError> {
    let symbol = &config.market.symbol;
    enumerate_timeframes_for_mode(config.operation)
        .iter()
        .map(|timeframe| build_bot_definition(strategy, timeframe, symbol, config.operation))
        .collect()
}

pub async fn persist_catalog_snapshot(
    config: &Config,
    strategy: &impl StrategySpec,
    store: &mut impl BotCatalogStore,
) -> Result<Vec<BotDefinition>, BotsError> {
    let entries = build_catalog_from_config(config, strategy)?;
    store
        .save_catalog(&entries)
        .await
        .map_err(BotsError::CatalogStore)?;
    Ok(entries)
}
