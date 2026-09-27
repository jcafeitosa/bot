use serde::{Deserialize, Serialize};

use super::{
    error::BotsError,
    identity::{normalize_symbol, BotId, StrategyId, StrategyVersion},
    run::RunId,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvaluationWindow {
    pub start_ms: i64,
    pub end_ms: i64,
}

impl EvaluationWindow {
    pub fn validate(&self) -> Result<(), BotsError> {
        if self.start_ms < 0 || self.end_ms <= self.start_ms {
            return Err(BotsError::InvalidWindow);
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
    pub fn validate(&self) -> Result<(), BotsError> {
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
            return Err(BotsError::InvalidMetrics);
        }
        Ok(())
    }
}
