use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::config::Config;
use crate::core::error::BotError;
use crate::modules::backtest::models::StrategyDefinition;
use crate::modules::bots::{
    bot_id_matches_market, build_catalog_from_monitor_registry, full_ranking,
    persist_monitor_catalog_snapshot, BotCatalogStore, BotDefinition, BotIdentity, BotMetrics,
    BotRankingReport, MonitorStrategyRegistry,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct BotCatalogEntry {
    pub bot_id: String,
    pub strategy_id: String,
    pub strategy_version: u32,
    pub timeframe: String,
    pub symbol: String,
    pub operation: String,
    /// SMA fast period from the monitor strategy registry when resolvable.
    pub monitor_fast_period: u32,
    /// SMA slow period from the monitor strategy registry when resolvable.
    pub monitor_slow_period: u32,
    /// Monitor evaluator kind (`sma_cross` or `ema_cross`) from the strategy registry.
    pub monitor_evaluator: String,
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

fn map_bot(def: BotDefinition, strategy: &StrategyDefinition) -> BotCatalogEntry {
    BotCatalogEntry {
        bot_id: def.id.to_string(),
        strategy_id: def.strategy_id.to_string(),
        strategy_version: def.strategy_version.0,
        timeframe: def.timeframe,
        symbol: def.symbol,
        operation: format!("{:?}", def.operation),
        monitor_fast_period: strategy.fast_period as u32,
        monitor_slow_period: strategy.slow_period as u32,
        monitor_evaluator: strategy.evaluator.as_str().to_string(),
    }
}

fn map_bot_from_registry(def: BotDefinition, config: &Config) -> Result<BotCatalogEntry, BotError> {
    let identity = def
        .identity()
        .map_err(|error| BotError::Configuration(error.to_string()))?;
    let registry = MonitorStrategyRegistry::from_config(config)
        .map_err(|error| BotError::Configuration(error.to_string()))?;
    let strategy = registry
        .resolve(&identity)
        .map_err(|error| BotError::Configuration(error.to_string()))?;
    Ok(map_bot(def, strategy))
}

pub fn catalog_for_config(config: &Config) -> Result<BotCatalogResponse, BotError> {
    let registry = MonitorStrategyRegistry::from_config(config)
        .map_err(|e| BotError::Configuration(e.to_string()))?;
    let bots = build_catalog_from_monitor_registry(config, &registry)
        .map_err(|e| BotError::Configuration(e.to_string()))?
        .into_iter()
        .map(|def| map_bot_from_registry(def, config))
        .collect::<Result<Vec<_>, _>>()?;
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
    let defs = persist_monitor_catalog_snapshot(config, store)
        .await
        .map_err(|e| BotError::Configuration(e.to_string()))?;
    Ok(BotCatalogPersistResponse {
        bots: defs
            .into_iter()
            .map(|def| map_bot_from_registry(def, config))
            .collect::<Result<Vec<_>, _>>()?,
        persisted: true,
    })
}

pub async fn catalog_from_store(
    store: &impl BotCatalogStore,
    config: &Config,
) -> Result<BotCatalogResponse, BotError> {
    let defs = store
        .load_catalog()
        .await
        .map_err(BotError::Configuration)?;
    let bots = defs
        .into_iter()
        .map(|def| map_bot_from_registry(def, config))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BotCatalogResponse { bots })
}

fn canonical_bot_id_for_catalog(bot_id: &str) -> Result<String, BotError> {
    let identity = BotIdentity::parse_bot_id(bot_id)
        .map_err(|error| BotError::Configuration(error.to_string()))?;
    let id = identity
        .bot_id()
        .map_err(|error| BotError::Configuration(error.to_string()))?;
    Ok(id.to_string())
}

fn catalog_has_bot(catalog: &BotCatalogResponse, canonical_bot_id: &str) -> bool {
    catalog
        .bots
        .iter()
        .any(|entry| entry.bot_id == canonical_bot_id)
}

/// Fail-closed: promotion must reference a bot known to the catalog seam (persisted store or config-built catalog).
pub async fn assert_catalog_contains_bot(
    store: &impl BotCatalogStore,
    config: &Config,
    bot_id: &str,
) -> Result<(), BotError> {
    let trimmed = bot_id.trim();
    if trimmed.is_empty() {
        return Err(BotError::Configuration("bot_id must not be empty".into()));
    }
    let canonical = canonical_bot_id_for_catalog(trimmed)?;
    let from_store = catalog_from_store(store, config).await?;
    if catalog_has_bot(&from_store, &canonical) {
        return Ok(());
    }
    let from_config = catalog_for_config(config)?;
    if catalog_has_bot(&from_config, &canonical) {
        return Ok(());
    }
    Err(BotError::Configuration(format!(
        "bot_id {trimmed} is not in the bot catalog; persist catalog or use a configured bot id"
    )))
}

/// Catalog + active monitor market checks before `BotRuntimePort::promote`.
pub async fn assert_bot_promotion_allowed(
    store: &impl BotCatalogStore,
    config: &Config,
    bot_id: &str,
) -> Result<(), BotError> {
    assert_catalog_contains_bot(store, config, bot_id).await?;
    if !bot_id_matches_market(bot_id, &config.market.symbol, &config.market.timeframe) {
        return Err(BotError::Configuration(
            "promoted bot symbol/timeframe does not match active monitor market config".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod catalog_gate_tests {
    use super::*;
    use crate::modules::bots::InMemoryBotCatalogStore;

    #[test]
    fn catalog_for_config_includes_monitor_strategy_periods() {
        let config = Config::default();
        let catalog = catalog_for_config(&config).expect("catalog");
        let entry = catalog.bots.first().expect("bot");
        assert_eq!(entry.monitor_fast_period, config.strategy.sma_fast as u32);
        assert_eq!(entry.monitor_slow_period, config.strategy.sma_slow as u32);
    }

    #[test]
    fn catalog_for_config_lists_monitor_registry_v2_periods() {
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(crate::core::config::MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA crossover v2".into(),
                fast_period: 3,
                slow_period: 15,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
            });
        let catalog = catalog_for_config(&config).expect("catalog");
        let v2 = catalog
            .bots
            .iter()
            .find(|entry| entry.strategy_version == 2)
            .expect("sma-cross@2 row");
        assert_eq!(v2.monitor_fast_period, 3);
        assert_eq!(v2.monitor_slow_period, 15);
        assert_eq!(v2.monitor_evaluator, "sma_cross");
    }

    #[test]
    fn catalog_for_config_exposes_ema_evaluator_from_registry() {
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(crate::core::config::MonitorStrategyConfigEntry {
                id: "ema-cross".into(),
                version: 1,
                name: "EMA crossover".into(),
                fast_period: 4,
                slow_period: 12,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::EmaCross,
            });
        let catalog = catalog_for_config(&config).expect("catalog");
        let ema = catalog
            .bots
            .iter()
            .find(|entry| entry.strategy_id == "ema-cross")
            .expect("ema-cross row");
        assert_eq!(ema.monitor_evaluator, "ema_cross");
        assert_eq!(ema.monitor_fast_period, 4);
    }

    #[test]
    fn catalog_for_config_exposes_ema_evaluator_on_registry_v2_row() {
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(crate::core::config::MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA id v2 EMA evaluator".into(),
                fast_period: 8,
                slow_period: 22,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::EmaCross,
            });
        let catalog = catalog_for_config(&config).expect("catalog");
        let v2 = catalog
            .bots
            .iter()
            .find(|entry| entry.strategy_version == 2)
            .expect("sma-cross@2 row");
        assert_eq!(v2.monitor_evaluator, "ema_cross");
        assert_eq!(v2.monitor_fast_period, 8);
    }

    #[tokio::test]
    async fn assert_catalog_contains_bot_accepts_config_materialized_id() {
        let config = Config::default();
        let store = InMemoryBotCatalogStore::new();
        let catalog = catalog_for_config(&config).expect("catalog");
        let bot_id = catalog.bots.first().expect("bot").bot_id.clone();
        assert_catalog_contains_bot(&store, &config, &bot_id)
            .await
            .expect("known bot");
    }

    #[tokio::test]
    async fn assert_catalog_contains_bot_accepts_compact_symbol_encoding() {
        let config = Config::default();
        let store = InMemoryBotCatalogStore::new();
        assert_catalog_contains_bot(&store, &config, "sma-cross@1:5m:BTCUSDT")
            .await
            .expect("compact symbol matches catalog");
    }

    #[tokio::test]
    async fn assert_catalog_contains_bot_rejects_unknown_id() {
        let config = Config::default();
        let store = InMemoryBotCatalogStore::new();
        let err = assert_catalog_contains_bot(&store, &config, "unknown@1:5m:BTCUSDT")
            .await
            .expect_err("unknown");
        assert!(err.to_string().contains("not in the bot catalog"));
    }

    #[tokio::test]
    async fn assert_bot_promotion_allowed_rejects_timeframe_mismatch() {
        let config = Config::default();
        let store = InMemoryBotCatalogStore::new();
        let err = assert_bot_promotion_allowed(&store, &config, "sma-cross@1:5m:BTCUSDT")
            .await
            .expect_err("wrong timeframe for bundled 15m config");
        assert!(err.to_string().contains("market config"));
    }

    #[tokio::test]
    async fn assert_bot_promotion_allowed_accepts_monitor_registry_v2() {
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(crate::core::config::MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA crossover v2".into(),
                fast_period: 3,
                slow_period: 15,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
            });
        let store = InMemoryBotCatalogStore::new();
        assert_bot_promotion_allowed(&store, &config, "sma-cross@2:15m:BTCUSDT")
            .await
            .expect("v2 bot materialized from monitor registry");
    }
}
