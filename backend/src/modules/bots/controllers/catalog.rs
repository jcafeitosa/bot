#![allow(dead_code)]

use crate::core::config::{Config, OperationMode};
use crate::modules::bots::adapters::BotCatalogStore;
use crate::modules::bots::controllers::monitor_strategy::MonitorStrategyRegistry;
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

/// Materializes one bot per (registered strategy × supported timeframe) for the active market symbol.
pub fn build_catalog_from_monitor_registry(
    config: &Config,
    registry: &MonitorStrategyRegistry,
) -> Result<Vec<BotDefinition>, BotsError> {
    let symbol = &config.market.symbol;
    let mut entries = Vec::new();
    for strategy in registry.definitions() {
        for timeframe in enumerate_timeframes_for_mode(config.operation) {
            entries.push(build_bot_definition(
                strategy,
                timeframe,
                symbol,
                config.operation,
            )?);
        }
    }
    Ok(entries)
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

pub async fn persist_monitor_catalog_snapshot(
    config: &Config,
    store: &mut impl BotCatalogStore,
) -> Result<Vec<BotDefinition>, BotsError> {
    let registry = MonitorStrategyRegistry::from_config(config)?;
    let entries = build_catalog_from_monitor_registry(config, &registry)?;
    store
        .save_catalog(&entries)
        .await
        .map_err(BotsError::CatalogStore)?;
    Ok(entries)
}

#[cfg(test)]
mod catalog_registry_tests {
    use std::collections::HashSet;

    use super::*;
    use crate::core::config::Config;
    use crate::modules::backtest::models::StrategyDefinition;
    use crate::modules::bots::controllers::monitor_strategy::monitor_strategy_from_config;
    use crate::modules::bots::models::{MonitorEvaluatorKind, StrategyId, StrategyVersion};

    #[test]
    fn build_catalog_from_monitor_registry_includes_each_registered_strategy() {
        let config = Config::default();
        let v1 = monitor_strategy_from_config(&config).expect("v1");
        let v2 = StrategyDefinition {
            id: StrategyId::new("sma-cross").expect("id"),
            version: StrategyVersion(2),
            name: "SMA crossover v2".into(),
            fast_period: 5,
            slow_period: 20,
            evaluator: MonitorEvaluatorKind::default(),
        };
        let registry = MonitorStrategyRegistry::from_definitions(vec![v1, v2]).expect("registry");
        let catalog = build_catalog_from_monitor_registry(&config, &registry).expect("catalog");
        let timeframes = enumerate_timeframes_for_mode(config.operation).len();
        assert_eq!(catalog.len(), timeframes * 2);
        let versions: HashSet<u32> = catalog.iter().map(|def| def.strategy_version.0).collect();
        assert!(versions.contains(&1) && versions.contains(&2));
    }
}
