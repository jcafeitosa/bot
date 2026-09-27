use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

use crate::core::config::OperationMode;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StrategyId(String);

impl StrategyId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > 80 {
            return Err(DomainError::InvalidId(
                "strategy id must be 1..=80 bytes".into(),
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StrategyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StrategyVersion(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BotId(String);

impl BotId {
    pub fn new(
        strategy: &StrategyId,
        version: StrategyVersion,
        timeframe: &str,
        symbol: &str,
    ) -> Result<Self, DomainError> {
        if !OperationMode::all_timeframes().contains(&timeframe) {
            return Err(DomainError::InvalidTimeframe(timeframe.to_owned()));
        }
        let symbol = normalize_symbol(symbol)?;
        Ok(Self(format!(
            "{}@{}:{}:{}",
            strategy.as_str(),
            version.0,
            timeframe,
            symbol
        )))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn normalize_symbol(symbol: &str) -> Result<String, DomainError> {
    let value = symbol.trim().to_ascii_uppercase();
    let mut parts = value.split('/');
    let base = parts.next().unwrap_or_default();
    let quote = parts.next().unwrap_or_default();
    let valid = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric());
    if !valid(base) || !valid(quote) || parts.next().is_some() || value.len() > 32 {
        return Err(DomainError::InvalidSymbol(value));
    }
    Ok(value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDefinition {
    pub id: StrategyId,
    pub version: StrategyVersion,
    pub name: String,
    pub fast_period: usize,
    pub slow_period: usize,
}

impl StrategyDefinition {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty()
            || self.fast_period == 0
            || self.fast_period >= self.slow_period
        {
            return Err(DomainError::InvalidStrategy(
                "name must be set and periods require 0 < fast < slow".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotDefinition {
    pub id: BotId,
    pub strategy_id: StrategyId,
    pub strategy_version: StrategyVersion,
    pub timeframe: String,
    pub symbol: String,
    pub operation: OperationMode,
}

impl BotDefinition {
    pub fn new(
        strategy: &StrategyDefinition,
        timeframe: &str,
        symbol: &str,
        operation: OperationMode,
    ) -> Result<Self, DomainError> {
        strategy.validate()?;
        if !operation.supported_timeframes().contains(&timeframe) {
            return Err(DomainError::InvalidTimeframeForMode {
                timeframe: timeframe.to_owned(),
                operation,
            });
        }
        let symbol = normalize_symbol(symbol)?;
        let id = BotId::new(&strategy.id, strategy.version, timeframe, &symbol)?;
        Ok(Self {
            id,
            strategy_id: strategy.id.clone(),
            strategy_version: strategy.version,
            timeframe: timeframe.to_owned(),
            symbol,
            operation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RunId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvaluationWindow {
    pub start_ms: i64,
    pub end_ms: i64,
}

impl EvaluationWindow {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.start_ms < 0 || self.end_ms <= self.start_ms {
            return Err(DomainError::InvalidWindow);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotMetrics {
    pub bot_id: BotId,
    pub timeframe: String,
    pub symbol: String,
    pub strategy_id: StrategyId,
    pub strategy_version: StrategyVersion,
    pub run_id: RunId,
    pub dataset_hash: String,
    pub window: EvaluationWindow,
    pub quote_currency: String,
    pub initial_capital_quote: f64,
    pub net_pnl_quote: f64,
    pub net_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub trades: usize,
    pub accuracy_pct: Option<f64>,
}

impl BotMetrics {
    pub fn validate(&self) -> Result<(), DomainError> {
        self.window.validate()?;
        if self.dataset_hash.trim().is_empty()
            || self.quote_currency.trim().is_empty()
            || !self.initial_capital_quote.is_finite()
            || self.initial_capital_quote <= 0.0
            || !self.net_pnl_quote.is_finite()
            || !self.net_return_pct.is_finite()
            || !self.max_drawdown_pct.is_finite()
            || !(0.0..=100.0).contains(&self.max_drawdown_pct)
            || self
                .accuracy_pct
                .is_some_and(|v| !v.is_finite() || !(0.0..=100.0).contains(&v))
            || normalize_symbol(&self.symbol).is_err()
            || BotId::new(
                &self.strategy_id,
                self.strategy_version,
                &self.timeframe,
                &self.symbol,
            )
            .ok()
            .as_ref()
                != Some(&self.bot_id)
        {
            return Err(DomainError::InvalidMetrics);
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
    /// Capital-weighted average of constituent bot maximum drawdowns; not portfolio peak-to-trough drawdown.
    pub constituent_weighted_drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRanking {
    pub rows: Vec<BotMetrics>,
}

pub fn rank_bots(rows: impl IntoIterator<Item = BotMetrics>) -> Result<BotRanking, DomainError> {
    let mut unique = BTreeMap::new();
    let mut scope: Option<(EvaluationWindow, String, String)> = None;
    for row in rows {
        row.validate()?;
        let row_scope = (
            row.window.clone(),
            row.quote_currency.to_ascii_uppercase(),
            row.dataset_hash.clone(),
        );
        if scope.as_ref().is_some_and(|current| current != &row_scope) {
            return Err(DomainError::IncompatibleRanking);
        }
        scope = Some(row_scope);
        let key = (
            row.bot_id.clone(),
            row.window.clone(),
            row.dataset_hash.clone(),
        );
        if unique.insert(key, row).is_some() {
            return Err(DomainError::DuplicateRun);
        }
    }
    let mut rows: Vec<_> = unique.into_values().collect();
    rows.sort_by(|a, b| {
        b.net_pnl_quote
            .total_cmp(&a.net_pnl_quote)
            .then_with(|| a.max_drawdown_pct.total_cmp(&b.max_drawdown_pct))
            .then_with(|| a.bot_id.cmp(&b.bot_id))
    });
    Ok(BotRanking { rows })
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
        row.validate()?;
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
        // Portfolio equity-curve drawdown requires timestamped equity series. Until persisted results include them,
        // this reports a capital-weighted mean of per-bot max drawdowns rather than pretending it's portfolio max drawdown.
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
        operation: OperationMode,
    },
    InvalidSymbol(String),
    InvalidStrategy(String),
    InvalidWindow,
    InvalidMetrics,
    DuplicateRun,
    IncompatibleRanking,
    AggregateOverflow,
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
    fn identity_is_strategy_version_timeframe_and_symbol() {
        let a = metric("sma-cross", "5m", "r1", 20.0);
        let b = metric("sma-cross", "15m", "r2", 10.0);
        assert_ne!(a.bot_id, b.bot_id);
    }
    #[test]
    fn rejects_malformed_market_symbols() {
        let id = StrategyId::new("sma-cross").unwrap();
        for symbol in ["/", "BTC/", "/USDT", "BTC//USDT"] {
            assert!(BotId::new(&id, StrategyVersion(1), "5m", symbol).is_err());
        }
    }
    #[test]
    fn bot_metrics_identity_fields_must_match() {
        let mut row = metric("sma-cross", "5m", "r1", 20.0);
        row.strategy_id = StrategyId::new("other").unwrap();
        assert_eq!(row.validate().unwrap_err(), DomainError::InvalidMetrics);
    }
    #[test]
    fn strategy_ranking_output_is_partitioned_by_currency_dataset_and_window() {
        let a = metric("sma-cross", "5m", "r1", 20.0);
        let mut b = metric("sma-cross", "15m", "r2", 10.0);
        b.quote_currency = "JPY".into();
        let ranks = rank_strategies([a.clone(), b.clone()]).unwrap();
        assert_eq!(
            ranks.len(),
            2,
            "distinct currency scopes remain separate ranking rows"
        );
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
    fn bot_ranking_orders_by_net_pnl() {
        let rank = rank_bots([
            metric("sma-cross", "5m", "r1", 20.0),
            metric("sma-cross", "15m", "r2", 10.0),
        ])
        .unwrap();
        assert!(rank.rows[0].net_pnl_quote > rank.rows[1].net_pnl_quote);
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
