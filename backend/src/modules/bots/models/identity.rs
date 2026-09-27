use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::BotsError;
use crate::core::config::OperationMode;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StrategyId(String);

impl StrategyId {
    pub fn new(value: impl Into<String>) -> Result<Self, BotsError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > 80 {
            return Err(BotsError::InvalidId(
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
    ) -> Result<Self, BotsError> {
        if !OperationMode::all_timeframes().contains(&timeframe) {
            return Err(BotsError::InvalidTimeframe(timeframe.to_owned()));
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

/// Structured bot identity: versioned strategy bound to timeframe and market symbol.
///
/// Distinct from [`crate::modules::agents::AgentId`] (administrative identity only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotIdentity {
    pub strategy_id: StrategyId,
    pub strategy_version: StrategyVersion,
    pub timeframe: String,
    pub symbol: String,
}

impl BotIdentity {
    pub fn new(
        strategy_id: StrategyId,
        strategy_version: StrategyVersion,
        timeframe: impl Into<String>,
        symbol: impl Into<String>,
    ) -> Result<Self, BotsError> {
        let timeframe = timeframe.into();
        if !OperationMode::all_timeframes().contains(&timeframe.as_str()) {
            return Err(BotsError::InvalidTimeframe(timeframe));
        }
        let symbol = normalize_symbol(&symbol.into())?;
        Ok(Self {
            strategy_id,
            strategy_version,
            timeframe,
            symbol,
        })
    }

    pub fn bot_id(&self) -> Result<BotId, BotsError> {
        BotId::new(
            &self.strategy_id,
            self.strategy_version,
            &self.timeframe,
            &self.symbol,
        )
    }
}

pub(crate) fn normalize_symbol(symbol: &str) -> Result<String, BotsError> {
    let value = symbol.trim().to_ascii_uppercase();
    let mut parts = value.split('/');
    let base = parts.next().unwrap_or_default();
    let quote = parts.next().unwrap_or_default();
    let valid = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric());
    if !valid(base) || !valid(quote) || parts.next().is_some() || value.len() > 32 {
        return Err(BotsError::InvalidSymbol(value));
    }
    Ok(value)
}
