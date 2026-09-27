use crate::core::config::Config;
use crate::modules::bots::adapters::{shared_bot_runtime, BotRuntimePort};
use crate::modules::bots::controllers::monitor_strategy::monitor_evaluation_for_promoted_identity;
use crate::modules::bots::models::bot_id_matches_market;
use crate::modules::bots::models::MonitorEvaluatorKind;
use crate::modules::bots::models::{BotIdentity, BotsError};

/// SMA/timeframe inputs for the monitor evaluation loop when bot runtime promotion is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyEvaluationBinding {
    pub evaluator: MonitorEvaluatorKind,
    pub sma_fast: usize,
    pub sma_slow: usize,
    pub timeframe: String,
    pub promoted_bot_id: Option<String>,
}

pub fn strategy_evaluation_binding(config: &Config) -> StrategyEvaluationBinding {
    strategy_evaluation_binding_with_runtime(config, shared_bot_runtime().as_ref())
}

pub fn strategy_evaluation_binding_with_runtime(
    config: &Config,
    runtime: &dyn BotRuntimePort,
) -> StrategyEvaluationBinding {
    let fallback = StrategyEvaluationBinding {
        evaluator: MonitorEvaluatorKind::SmaCross,
        sma_fast: config.strategy.sma_fast,
        sma_slow: config.strategy.sma_slow,
        timeframe: config.market.timeframe.clone(),
        promoted_bot_id: None,
    };
    let status = runtime.status();
    if !status.runtime_enabled {
        return fallback;
    }
    let active = match status.active {
        Some(record) => record,
        None => return fallback,
    };
    match promotion_matches_monitor(&active.bot_id, config) {
        Ok(identity) => match monitor_evaluation_for_promoted_identity(&identity, config) {
            Ok(spec) => StrategyEvaluationBinding {
                evaluator: spec.evaluator,
                sma_fast: spec.fast_period,
                sma_slow: spec.slow_period,
                timeframe: identity.timeframe,
                promoted_bot_id: Some(active.bot_id),
            },
            Err(error) => {
                tracing::warn!(
                    target: "bots",
                    bot_id = active.bot_id,
                    ?error,
                    "active bot promotion ignored: unknown strategy for monitor evaluator"
                );
                fallback
            }
        },
        Err(error) => {
            tracing::warn!(
                target: "bots",
                bot_id = active.bot_id,
                ?error,
                "active bot promotion ignored for strategy evaluation"
            );
            fallback
        }
    }
}

fn promotion_matches_monitor(bot_id: &str, config: &Config) -> Result<BotIdentity, BotsError> {
    if !bot_id_matches_market(bot_id, &config.market.symbol, &config.market.timeframe) {
        return Err(BotsError::InvalidId(
            "promoted bot symbol/timeframe does not match monitor config".into(),
        ));
    }
    BotIdentity::parse_bot_id(bot_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;
    use crate::modules::bots::adapters::InMemoryBotRuntime;
    use crate::modules::bots::models::{PromoteBotRequest, StrategyId, StrategyVersion};

    #[test]
    fn strategy_evaluation_binding_without_runtime_uses_config() {
        std::env::remove_var("BOT_RUNTIME_ENABLED");
        let config = Config::default();
        let runtime = InMemoryBotRuntime::new();
        let binding = strategy_evaluation_binding_with_runtime(&config, &runtime);
        assert_eq!(binding.sma_fast, config.strategy.sma_fast);
        assert_eq!(binding.sma_slow, config.strategy.sma_slow);
        assert_eq!(binding.timeframe, config.market.timeframe);
        assert!(binding.promoted_bot_id.is_none());
    }

    #[test]
    fn strategy_evaluation_binding_with_promotion_sets_bot_id_and_periods() {
        let config = Config::default();
        let runtime = InMemoryBotRuntime::new();
        let bot_id = BotIdentity::new(
            StrategyId::new("sma-cross").unwrap(),
            StrategyVersion(1),
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap()
        .bot_id()
        .unwrap()
        .to_string();
        let bot_id_for_assert = bot_id.clone();
        runtime
            .promote(PromoteBotRequest {
                bot_id,
                promoted_by: "test-owner".into(),
            })
            .expect("promote");
        let binding = strategy_evaluation_binding_with_runtime(&config, &runtime);
        assert_eq!(
            binding.promoted_bot_id.as_deref(),
            Some(bot_id_for_assert.as_str())
        );
        assert_eq!(binding.sma_fast, config.strategy.sma_fast);
        assert_eq!(binding.sma_slow, config.strategy.sma_slow);
        assert_eq!(binding.timeframe, config.market.timeframe);
    }

    #[test]
    fn strategy_evaluation_binding_with_v2_promotion_uses_registry_periods() {
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
        let runtime = InMemoryBotRuntime::new();
        let bot_id = BotIdentity::new(
            StrategyId::new("sma-cross").unwrap(),
            StrategyVersion(2),
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap()
        .bot_id()
        .unwrap()
        .to_string();
        runtime
            .promote(PromoteBotRequest {
                bot_id,
                promoted_by: "test-owner".into(),
            })
            .expect("promote");
        let binding = strategy_evaluation_binding_with_runtime(&config, &runtime);
        assert_eq!(binding.sma_fast, 3);
        assert_eq!(binding.sma_slow, 15);
        assert_eq!(binding.evaluator, MonitorEvaluatorKind::SmaCross);
        assert!(binding.promoted_bot_id.is_some());
    }

    #[test]
    fn strategy_evaluation_binding_uses_ema_evaluator_from_registry() {
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
                evaluator: MonitorEvaluatorKind::EmaCross,
            });
        let runtime = InMemoryBotRuntime::new();
        let bot_id = BotIdentity::new(
            StrategyId::new("ema-cross").unwrap(),
            StrategyVersion(1),
            config.market.timeframe.as_str(),
            config.market.symbol.as_str(),
        )
        .unwrap()
        .bot_id()
        .unwrap()
        .to_string();
        runtime
            .promote(PromoteBotRequest {
                bot_id,
                promoted_by: "test-owner".into(),
            })
            .expect("promote");
        let binding = strategy_evaluation_binding_with_runtime(&config, &runtime);
        assert_eq!(binding.evaluator, MonitorEvaluatorKind::EmaCross);
        assert_eq!(binding.sma_fast, 4);
        assert_eq!(binding.sma_slow, 12);
    }

    #[test]
    fn bot_identity_parse_accepts_compact_symbol_in_bot_id() {
        let parsed = BotIdentity::parse_bot_id("sma-cross@1:5m:BTCUSDT").unwrap();
        assert_eq!(parsed.symbol, "BTC/USDT");
        assert_eq!(parsed.timeframe, "5m");
    }

    #[test]
    fn bot_identity_parse_round_trip() {
        let id = BotIdentity::new(
            StrategyId::new("sma-cross").unwrap(),
            StrategyVersion(1),
            "5m",
            "BTC/USDT",
        )
        .unwrap()
        .bot_id()
        .unwrap();
        let parsed = BotIdentity::parse_bot_id(id.as_str()).unwrap();
        assert_eq!(parsed.timeframe, "5m");
        assert_eq!(parsed.symbol, "BTC/USDT");
    }
}
