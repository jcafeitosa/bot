pub mod account_file;
pub mod binance;
pub mod bootstrap;
pub mod capabilities;
pub mod live;
pub mod market_data;
pub mod preflight;
pub mod registry;
pub mod resources;
pub mod rest;
pub mod router;
pub mod stream;
pub mod ws;

pub use market_data::MarketDataSource;

use serde::{Deserialize, Serialize};

use crate::config::Environment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExchangeId {
    Binance,
}

impl std::fmt::Display for ExchangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binance => write!(f, "binance"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketType {
    Spot,
    Futures,
}

impl std::fmt::Display for MarketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spot => write!(f, "spot"),
            Self::Futures => write!(f, "futures"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExchangeAccountId {
    pub exchange: ExchangeId,
    pub market: MarketType,
    pub account_label: String,
    pub environment: Environment,
}

impl ExchangeAccountId {
    pub fn new(
        exchange: ExchangeId,
        market: MarketType,
        account_label: impl Into<String>,
        environment: Environment,
    ) -> Result<Self, ExchangeError> {
        let account_label = account_label.into().trim().to_owned();
        if account_label.is_empty() || account_label.len() > 64 {
            return Err(ExchangeError::InvalidAccountLabel);
        }
        Ok(Self {
            exchange,
            market,
            account_label,
            environment,
        })
    }

    pub fn key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.exchange, self.market, self.account_label, self.environment
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Transport {
    Rest,
    StreamWs,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ExchangeError {
    #[error("invalid account label")]
    InvalidAccountLabel,
    #[error("account not registered")]
    UnknownAccount,
    #[error("market type not supported for this account")]
    UnsupportedMarket,
    #[error("order execution is disabled in this build")]
    ExecutionDisabled,
    #[error("stream disconnected")]
    StreamDisconnected,
    #[error("configured endpoint is outside the permitted Spot testnet origin")]
    InvalidEndpoint,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;

    #[test]
    fn account_key_is_stable_and_unique_per_market() {
        let spot = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Spot,
            "main",
            Environment::Dev,
        )
        .unwrap();
        let futures = ExchangeAccountId::new(
            ExchangeId::Binance,
            MarketType::Futures,
            "main",
            Environment::Dev,
        )
        .unwrap();
        assert_ne!(spot.key(), futures.key());
    }

    #[test]
    fn rejects_empty_account_label() {
        assert_eq!(
            ExchangeAccountId::new(ExchangeId::Binance, MarketType::Spot, " ", Environment::Dev)
                .unwrap_err(),
            ExchangeError::InvalidAccountLabel
        );
    }
}
