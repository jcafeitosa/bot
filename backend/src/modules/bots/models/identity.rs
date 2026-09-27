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
        let symbol = normalize_symbol_segment(&symbol.into())?;
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

    /// Parses `strategy@version:timeframe:symbol` (same encoding as [`BotId::new`]).
    pub fn parse_bot_id(value: &str) -> Result<Self, BotsError> {
        let trimmed = value.trim();
        let (strategy, rest) = trimmed.split_once('@').ok_or_else(|| {
            BotsError::InvalidId("bot id must be strategy@version:timeframe:symbol".into())
        })?;
        let mut parts = rest.splitn(3, ':');
        let version_raw = parts
            .next()
            .ok_or_else(|| BotsError::InvalidId("bot id missing version".into()))?;
        let timeframe = parts
            .next()
            .ok_or_else(|| BotsError::InvalidId("bot id missing timeframe".into()))?;
        let symbol = parts
            .next()
            .ok_or_else(|| BotsError::InvalidId("bot id missing symbol".into()))?;
        let version = version_raw
            .parse::<u32>()
            .map_err(|_| BotsError::InvalidId("bot id version must be u32".into()))?;
        Self::new(
            StrategyId::new(strategy)?,
            StrategyVersion(version),
            timeframe,
            symbol,
        )
    }
}

impl BotId {
    /// Parses the canonical bot id string (public seam for catalog/HTTP).
    #[allow(dead_code)]
    pub fn parse(value: &str) -> Result<Self, BotsError> {
        BotIdentity::parse_bot_id(value)?.bot_id()
    }
}

pub(crate) fn normalize_symbol_segment(symbol: &str) -> Result<String, BotsError> {
    if symbol.contains('/') {
        return normalize_symbol(symbol);
    }
    let compact = symbol.trim().to_ascii_uppercase();
    const QUOTES: &[&str] = &["USDT", "USDC", "BUSD", "FDUSD", "BTC", "ETH", "BNB"];
    for quote in QUOTES {
        if compact.len() > quote.len() && compact.ends_with(quote) {
            let base = &compact[..compact.len() - quote.len()];
            if !base.is_empty() {
                return normalize_symbol(&format!("{}/{}", base, quote));
            }
        }
    }
    normalize_symbol(symbol)
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

/// Compares monitor market symbol with the symbol segment of a canonical `BotId` string.
pub fn market_symbols_equivalent(bot_id_symbol: &str, market_symbol: &str) -> bool {
    fn compact(symbol: &str) -> String {
        symbol.trim().to_ascii_uppercase().replace('/', "")
    }
    if compact(bot_id_symbol) == compact(market_symbol) {
        return true;
    }
    match (
        normalize_symbol(bot_id_symbol),
        normalize_symbol(market_symbol),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Returns true when `bot_id` is `strategy@version:timeframe:symbol` and tail matches market.
pub fn bot_id_matches_market(bot_id: &str, market_symbol: &str, market_timeframe: &str) -> bool {
    let Some(tail) = bot_id.split('@').nth(1) else {
        return false;
    };
    let parts: Vec<&str> = tail.splitn(3, ':').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[1] == market_timeframe && market_symbols_equivalent(parts[2], market_symbol)
}
