use crate::core::config::Config;
use crate::modules::backtest::models::StrategyDefinition;
use crate::modules::bots::models::{
    BotIdentity, BotsError, MonitorEvaluatorKind, StrategyId, StrategyVersion,
};

/// Registered strategy definitions the monitor evaluator may bind at runtime (extensible list).
#[derive(Debug, Clone)]
pub struct MonitorStrategyRegistry {
    entries: Vec<StrategyDefinition>,
}

impl MonitorStrategyRegistry {
    pub fn from_config(config: &Config) -> Result<Self, BotsError> {
        let mut definitions = vec![monitor_strategy_from_config(config)?];
        for entry in &config.strategy.monitor_registry {
            definitions.push(StrategyDefinition {
                id: StrategyId::new(&entry.id)?,
                version: StrategyVersion(entry.version),
                name: entry.name.clone(),
                fast_period: entry.fast_period,
                slow_period: entry.slow_period,
                evaluator: entry.evaluator,
            });
        }
        Self::from_definitions(definitions)
    }

    pub fn from_definitions(definitions: Vec<StrategyDefinition>) -> Result<Self, BotsError> {
        for definition in &definitions {
            definition.validate()?;
        }
        for (index, left) in definitions.iter().enumerate() {
            for right in definitions.iter().skip(index + 1) {
                if left.id.as_str() == right.id.as_str() && left.version == right.version {
                    return Err(BotsError::InvalidStrategy(format!(
                        "duplicate monitor strategy registration {} v{}",
                        left.id, left.version.0
                    )));
                }
            }
        }
        Ok(Self {
            entries: definitions,
        })
    }

    pub fn definitions(&self) -> &[StrategyDefinition] {
        &self.entries
    }

    pub fn resolve(&self, identity: &BotIdentity) -> Result<&StrategyDefinition, BotsError> {
        self.entries
            .iter()
            .find(|entry| {
                entry.id.as_str() == identity.strategy_id.as_str()
                    && entry.version == identity.strategy_version
            })
            .ok_or_else(|| {
                BotsError::InvalidStrategy(format!(
                    "monitor evaluator not registered for strategy {} v{}",
                    identity.strategy_id, identity.strategy_version.0
                ))
            })
    }
}

/// Canonical monitor/catalog strategy materialized from config (matches HTTP catalog seam).
pub fn monitor_strategy_from_config(config: &Config) -> Result<StrategyDefinition, BotsError> {
    Ok(StrategyDefinition {
        id: StrategyId::new("sma-cross")?,
        version: StrategyVersion(1),
        name: "SMA crossover".into(),
        fast_period: config.strategy.sma_fast,
        slow_period: config.strategy.sma_slow,
        evaluator: MonitorEvaluatorKind::SmaCross,
    })
}

/// Resolved monitor evaluation inputs for a promoted bot identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorEvaluationSpec {
    pub evaluator: MonitorEvaluatorKind,
    pub fast_period: usize,
    pub slow_period: usize,
}

pub fn monitor_evaluation_for_promoted_identity(
    identity: &BotIdentity,
    config: &Config,
) -> Result<MonitorEvaluationSpec, BotsError> {
    let registry = MonitorStrategyRegistry::from_config(config)?;
    let strategy = registry.resolve(identity)?;
    Ok(MonitorEvaluationSpec {
        evaluator: strategy.evaluator,
        fast_period: strategy.fast_period,
        slow_period: strategy.slow_period,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;

    #[test]
    fn monitor_strategy_registry_includes_configured_extensions() {
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
                evaluator: MonitorEvaluatorKind::default(),
            });
        let registry = MonitorStrategyRegistry::from_config(&config).expect("registry");
        assert_eq!(registry.definitions().len(), 2);
        let identity = BotIdentity::new(
            StrategyId::new("sma-cross").unwrap(),
            StrategyVersion(2),
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap();
        let spec = monitor_evaluation_for_promoted_identity(&identity, &config).expect("v2");
        assert_eq!(spec.fast_period, 3);
        assert_eq!(spec.slow_period, 15);
    }

    #[test]
    fn monitor_strategy_registry_from_config_lists_sma_cross_v1() {
        let config = Config::default();
        let registry = MonitorStrategyRegistry::from_config(&config).expect("registry");
        assert_eq!(registry.definitions().len(), 1);
        let entry = registry.definitions().first().expect("sma-cross");
        assert_eq!(entry.id.as_str(), "sma-cross");
        assert_eq!(entry.version, StrategyVersion(1));
    }

    #[test]
    fn monitor_strategy_registry_resolves_multiple_versions() {
        let config = Config::default();
        let base = monitor_strategy_from_config(&config).expect("v1");
        let v2 = StrategyDefinition {
            id: StrategyId::new("sma-cross").expect("id"),
            version: StrategyVersion(2),
            name: "SMA crossover v2".into(),
            fast_period: 5,
            slow_period: 20,
            evaluator: MonitorEvaluatorKind::default(),
        };
        let registry =
            MonitorStrategyRegistry::from_definitions(vec![base.clone(), v2.clone()]).expect("two");
        let identity_v1 = BotIdentity::new(
            base.id.clone(),
            base.version,
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap();
        let identity_v2 = BotIdentity::new(
            v2.id.clone(),
            v2.version,
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap();
        assert_eq!(
            registry.resolve(&identity_v1).expect("v1").fast_period,
            config.strategy.sma_fast
        );
        assert_eq!(registry.resolve(&identity_v2).expect("v2").fast_period, 5);
    }

    #[test]
    fn promoted_identity_must_match_monitor_strategy_registry() {
        let config = Config::default();
        let identity = BotIdentity::new(
            StrategyId::new("unknown-strategy").unwrap(),
            StrategyVersion(1),
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap();
        assert!(monitor_evaluation_for_promoted_identity(&identity, &config).is_err());
    }

    #[test]
    fn promoted_sma_cross_identity_uses_config_periods() {
        let config = Config::default();
        let strategy = monitor_strategy_from_config(&config).expect("registry");
        let identity = BotIdentity::new(
            strategy.id.clone(),
            strategy.version,
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap();
        let spec =
            monitor_evaluation_for_promoted_identity(&identity, &config).expect("sma-cross@1");
        let (fast, slow) = (spec.fast_period, spec.slow_period);
        assert_eq!(fast, config.strategy.sma_fast);
        assert_eq!(slow, config.strategy.sma_slow);
    }

    #[test]
    fn promoted_ema_cross_from_registry_uses_configured_periods() {
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(crate::core::config::MonitorStrategyConfigEntry {
                id: "ema-cross".into(),
                version: 1,
                name: "EMA crossover".into(),
                fast_period: 7,
                slow_period: 21,
                evaluator: MonitorEvaluatorKind::EmaCross,
            });
        let identity = BotIdentity::new(
            StrategyId::new("ema-cross").unwrap(),
            StrategyVersion(1),
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap();
        let spec =
            monitor_evaluation_for_promoted_identity(&identity, &config).expect("ema-cross@1");
        assert_eq!(spec.evaluator, MonitorEvaluatorKind::EmaCross);
        assert_eq!(spec.fast_period, 7);
        assert_eq!(spec.slow_period, 21);
    }
}
