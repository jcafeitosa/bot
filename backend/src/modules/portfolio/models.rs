use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Asset(String);
impl Asset {
    pub fn new(value: impl Into<String>) -> Result<Self, PortfolioError> {
        let value = value.into().trim().to_ascii_uppercase();
        if value.is_empty() || value.len() > 16 || !value.bytes().all(|c| c.is_ascii_alphanumeric())
        {
            return Err(PortfolioError::InvalidAsset);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub asset: Asset,
    pub available: Decimal,
    pub locked: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub base_asset: Asset,
    pub quote_asset: Asset,
    pub quantity: Decimal,
    pub average_entry_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub balances: Vec<WalletBalance>,
    pub positions: Vec<Position>,
    pub realized_pnl_by_quote: BTreeMap<Asset, Decimal>,
    pub as_of_ms: i64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PortfolioError {
    #[error("invalid asset code")]
    InvalidAsset,
    #[error("snapshot timestamp must be non-negative")]
    InvalidTimestamp,
    #[error("balances cannot be negative")]
    NegativeBalance,
    #[error("position quantity and entry price must be positive")]
    InvalidPosition,
}
