use serde::{Deserialize, Serialize};

use super::{
    error::BotsError,
    identity::{normalize_symbol, BotId, BotIdentity, StrategyId, StrategyVersion},
};
use crate::core::config::OperationMode;

/// Minimal strategy metadata required to materialize a [`BotDefinition`].
pub trait StrategySpec {
    fn strategy_id(&self) -> &StrategyId;
    fn strategy_version(&self) -> StrategyVersion;
    fn validate_strategy(&self) -> Result<(), BotsError>;
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
        strategy: &impl StrategySpec,
        timeframe: &str,
        symbol: &str,
        operation: OperationMode,
    ) -> Result<Self, BotsError> {
        strategy.validate_strategy()?;
        if !operation.supported_timeframes().contains(&timeframe) {
            return Err(BotsError::InvalidTimeframeForMode {
                timeframe: timeframe.to_owned(),
                operation,
            });
        }
        let symbol = normalize_symbol(symbol)?;
        let identity = BotIdentity::new(
            strategy.strategy_id().clone(),
            strategy.strategy_version(),
            timeframe,
            &symbol,
        )?;
        let id = identity.bot_id()?;
        Ok(Self {
            id,
            strategy_id: strategy.strategy_id().clone(),
            strategy_version: strategy.strategy_version(),
            timeframe: timeframe.to_owned(),
            symbol,
            operation,
        })
    }

    pub fn identity(&self) -> Result<BotIdentity, BotsError> {
        BotIdentity::new(
            self.strategy_id.clone(),
            self.strategy_version,
            &self.timeframe,
            &self.symbol,
        )
    }
}
