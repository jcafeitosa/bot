use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

pub use crate::modules::bots::models::{
    BotDefinition, BotId, BotMetrics, BotRanking, EvaluationWindow, RunId, StrategyId,
    StrategyVersion,
};
use crate::modules::bots::models::{BotsError, StrategySpec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDefinition {
    pub id: StrategyId,
    pub version: StrategyVersion,
    pub name: String,
    pub fast_period: usize,
    pub slow_period: usize,
}

impl StrategySpec for StrategyDefinition {
    fn strategy_id(&self) -> &StrategyId {
        &self.id
    }

    fn strategy_version(&self) -> StrategyVersion {
        self.version
    }

    fn validate_strategy(&self) -> Result<(), BotsError> {
        StrategyDefinition::validate(self)
    }
}

impl StrategyDefinition {
    pub fn validate(&self) -> Result<(), BotsError> {
        if self.name.trim().is_empty()
            || self.fast_period == 0
            || self.fast_period >= self.slow_period
        {
            return Err(BotsError::InvalidStrategy(
                "name must be set and periods require 0 < fast < slow".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRanking {
    pub strategy_id: StrategyId,
    pub strategy_version: StrategyVersion,
    pub window: EvaluationWindow,
    pub dataset_hash: String,
    pub quote_currency: String,
    pub net_pnl_quote_sum: f64,
    pub bot_count: usize,
    pub total_trades: usize,
    pub constituent_weighted_drawdown_pct: f64,
}

pub fn rank_bots(rows: impl IntoIterator<Item = BotMetrics>) -> Result<BotRanking, DomainError> {
    crate::modules::bots::rank_bots(rows).map_err(DomainError::from)
}

pub fn rank_strategies(
    rows: impl IntoIterator<Item = BotMetrics>,
) -> Result<Vec<StrategyRanking>, DomainError> {
    let mut unique: BTreeMap<
        (
            StrategyId,
            StrategyVersion,
            EvaluationWindow,
            String,
            String,
            BotId,
        ),
        BotMetrics,
    > = BTreeMap::new();
    let mut run_ids = std::collections::BTreeSet::new();
    for row in rows {
        row.validate().map_err(DomainError::from)?;
        if !run_ids.insert(row.run_id.clone()) {
            return Err(DomainError::DuplicateRun);
        }
        let quote = row.quote_currency.to_ascii_uppercase();
        let key = (
            row.strategy_id.clone(),
            row.strategy_version,
            row.window.clone(),
            quote,
            row.dataset_hash.clone(),
            row.bot_id.clone(),
        );
        if unique.insert(key, row).is_some() {
            return Err(DomainError::DuplicateRun);
        }
    }
    let mut grouped: BTreeMap<
        (
            StrategyId,
            StrategyVersion,
            EvaluationWindow,
            String,
            String,
        ),
        Vec<BotMetrics>,
    > = BTreeMap::new();
    for ((strategy, version, window, quote, dataset, _bot), row) in unique {
        grouped
            .entry((strategy, version, window, quote, dataset))
            .or_default()
            .push(row);
    }
    let mut result = Vec::new();
    for ((strategy_id, strategy_version, window, quote_currency, dataset_hash), group) in grouped {
        let net_pnl_quote_sum = group.iter().try_fold(0.0f64, |sum, row| {
            let next = sum + row.net_pnl_quote;
            if next.is_finite() {
                Ok(next)
            } else {
                Err(DomainError::AggregateOverflow)
            }
        })?;
        let initial_capital = group.iter().try_fold(0.0f64, |sum, row| {
            let next = sum + row.initial_capital_quote;
            if next.is_finite() {
                Ok(next)
            } else {
                Err(DomainError::AggregateOverflow)
            }
        })?;
        let weighted_drawdown_value = group.iter().try_fold(0.0f64, |sum, row| {
            let next = sum + row.max_drawdown_pct * row.initial_capital_quote;
            if next.is_finite() {
                Ok(next)
            } else {
                Err(DomainError::AggregateOverflow)
            }
        })?;
        let aggregate_drawdown_pct = (weighted_drawdown_value / initial_capital).clamp(0.0, 100.0);
        if !aggregate_drawdown_pct.is_finite() {
            return Err(DomainError::AggregateOverflow);
        }
        let total_trades = group.iter().try_fold(0usize, |sum, row| {
            sum.checked_add(row.trades)
                .ok_or(DomainError::AggregateOverflow)
        })?;
        result.push(StrategyRanking {
            strategy_id,
            strategy_version,
            window,
            dataset_hash,
            quote_currency,
            net_pnl_quote_sum,
            bot_count: group.len(),
            total_trades,
            constituent_weighted_drawdown_pct: aggregate_drawdown_pct,
        });
    }
    result.sort_by(|a, b| {
        b.net_pnl_quote_sum
            .total_cmp(&a.net_pnl_quote_sum)
            .then_with(|| a.strategy_id.cmp(&b.strategy_id))
    });
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidId(String),
    InvalidTimeframe(String),
    InvalidTimeframeForMode {
        timeframe: String,
        operation: crate::core::config::OperationMode,
    },
    InvalidSymbol(String),
    InvalidStrategy(String),
    InvalidWindow,
    InvalidMetrics,
    DuplicateRun,
    IncompatibleRanking,
    CatalogStore(String),
    AggregateOverflow,
}

impl From<BotsError> for DomainError {
    fn from(value: BotsError) -> Self {
        match value {
            BotsError::InvalidId(s) => Self::InvalidId(s),
            BotsError::InvalidTimeframe(s) => Self::InvalidTimeframe(s),
            BotsError::InvalidTimeframeForMode {
                timeframe,
                operation,
            } => Self::InvalidTimeframeForMode {
                timeframe,
                operation,
            },
            BotsError::InvalidSymbol(s) => Self::InvalidSymbol(s),
            BotsError::InvalidStrategy(s) => Self::InvalidStrategy(s),
            BotsError::InvalidWindow => Self::InvalidWindow,
            BotsError::InvalidMetrics => Self::InvalidMetrics,
            BotsError::DuplicateRun => Self::DuplicateRun,
            BotsError::IncompatibleRanking => Self::IncompatibleRanking,
            BotsError::CatalogStore(message) => Self::CatalogStore(message),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DomainError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::bots::models::BotMetrics;

    fn metric(strategy: &str, timeframe: &str, run: &str, pnl: f64) -> BotMetrics {
        let strategy_id = StrategyId::new(strategy).unwrap();
        let bot_id = BotId::new(&strategy_id, StrategyVersion(1), timeframe, "BTC/USDT").unwrap();
        BotMetrics {
            bot_id,
            timeframe: timeframe.into(),
            symbol: "BTC/USDT".into(),
            strategy_id,
            strategy_version: StrategyVersion(1),
            run_id: RunId(run.into()),
            dataset_hash: "dataset-v1".into(),
            window: EvaluationWindow {
                start_ms: 100,
                end_ms: 200,
            },
            quote_currency: "USDT".into(),
            initial_capital_quote: 1000.0,
            net_pnl_quote: pnl,
            net_return_pct: pnl / 10.0,
            max_drawdown_pct: 5.0,
            trades: 10,
            accuracy_pct: Some(92.0),
        }
    }

    #[test]
    fn strategy_ranking_output_is_partitioned_by_currency_dataset_and_window() {
        let a = metric("sma-cross", "5m", "r1", 20.0);
        let mut b = metric("sma-cross", "15m", "r2", 10.0);
        b.quote_currency = "JPY".into();
        let ranks = rank_strategies([a.clone(), b.clone()]).unwrap();
        assert_eq!(ranks.len(), 2);
        assert_eq!(
            rank_bots([a.clone(), b]).unwrap_err(),
            DomainError::IncompatibleRanking
        );
    }

    #[test]
    fn strategy_ranking_sums_net_quote_pnl_across_timeframes() {
        let ranks = rank_strategies([
            metric("sma-cross", "5m", "r1", 20.0),
            metric("sma-cross", "15m", "r2", 10.0),
        ])
        .unwrap();
        assert_eq!(ranks.len(), 1);
        assert_eq!(ranks[0].bot_count, 2);
        assert_eq!(ranks[0].net_pnl_quote_sum, 30.0);
    }

    #[test]
    fn rejects_duplicate_run_ids_for_strategy_aggregation() {
        assert_eq!(
            rank_strategies([
                metric("sma-cross", "5m", "same", 1.0),
                metric("sma-cross", "15m", "same", 1.0)
            ])
            .unwrap_err(),
            DomainError::DuplicateRun
        );
    }
}
