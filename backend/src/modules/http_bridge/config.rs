use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::core::config::{Config, MonitorCli};

pub use crate::core::config::MonitorStrategyConfigEntry;
use crate::core::error::BotError;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ConfigSnapshotQuery {
    #[schema(example = "src/core/config/bot.toml")]
    pub config: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigSnapshotResponse {
    pub environment: String,
    pub operation: String,
    pub risk_profile: String,
    pub run_mode: String,
    pub symbol: String,
    pub timeframe: String,
    pub sma_fast: usize,
    pub sma_slow: usize,
    pub jev_enabled: bool,
    pub production_enabled: bool,
    /// Extra monitor/catalog strategies from `[[strategy.monitor_registry]]` (canonical `sma-cross@1` uses `sma_fast` / `sma_slow`).
    pub monitor_registry: Vec<MonitorStrategyConfigEntry>,
}

pub fn load_config_snapshot(
    query: ConfigSnapshotQuery,
) -> Result<ConfigSnapshotResponse, BotError> {
    let cli = MonitorCli {
        config: PathBuf::from(query.config),
        environment: None,
        operation: None,
        risk_profile: None,
        mode: None,
    };
    let config = Config::load(&cli)?;
    Ok(map_config(&config))
}

pub fn map_config(config: &Config) -> ConfigSnapshotResponse {
    ConfigSnapshotResponse {
        environment: format!("{:?}", config.environment),
        operation: format!("{:?}", config.operation),
        risk_profile: format!("{:?}", config.risk_profile),
        run_mode: format!("{:?}", config.run_mode),
        symbol: config.market.symbol.clone(),
        timeframe: config.market.timeframe.clone(),
        sma_fast: config.strategy.sma_fast,
        sma_slow: config.strategy.sma_slow,
        jev_enabled: config.jev.enabled,
        production_enabled: config.production.enabled,
        monitor_registry: config.strategy.monitor_registry.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::MonitorStrategyConfigEntry;

    #[test]
    fn map_config_includes_monitor_registry_extensions() {
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA crossover v2".into(),
                fast_period: 3,
                slow_period: 15,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
            });
        let snap = map_config(&config);
        assert_eq!(snap.monitor_registry.len(), 1);
        assert_eq!(snap.monitor_registry[0].version, 2);
        assert_eq!(snap.monitor_registry[0].fast_period, 3);
        assert_eq!(
            snap.monitor_registry[0].evaluator,
            crate::modules::bots::MonitorEvaluatorKind::SmaCross
        );
    }

    #[test]
    fn map_config_preserves_ema_evaluator_on_registry_entry() {
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(MonitorStrategyConfigEntry {
                id: "ema-cross".into(),
                version: 1,
                name: "EMA crossover".into(),
                fast_period: 4,
                slow_period: 12,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::EmaCross,
            });
        let snap = map_config(&config);
        assert_eq!(snap.monitor_registry[0].evaluator.as_str(), "ema_cross");
    }
}
