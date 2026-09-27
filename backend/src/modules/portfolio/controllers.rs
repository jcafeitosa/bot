use rust_decimal::Decimal;
use std::collections::BTreeMap;

use super::models::{Asset, PortfolioError, PortfolioSnapshot, WalletBalance};

pub fn paper_snapshot(quote: &Asset) -> PortfolioSnapshot {
    PortfolioSnapshot {
        balances: vec![WalletBalance {
            asset: quote.clone(),
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
        }],
        positions: vec![],
        realized_pnl_by_quote: BTreeMap::new(),
        as_of_ms: 0,
    }
}

impl PortfolioSnapshot {
    pub fn validate(&self) -> Result<(), PortfolioError> {
        if self.as_of_ms < 0 {
            return Err(PortfolioError::InvalidTimestamp);
        }
        if self
            .balances
            .iter()
            .any(|b| b.available < Decimal::ZERO || b.locked < Decimal::ZERO)
        {
            return Err(PortfolioError::NegativeBalance);
        }
        if self
            .positions
            .iter()
            .any(|p| p.quantity <= Decimal::ZERO || p.average_entry_price <= Decimal::ZERO)
        {
            return Err(PortfolioError::InvalidPosition);
        }
        Ok(())
    }

    pub fn balance(&self, asset: &Asset) -> Decimal {
        self.balances
            .iter()
            .filter(|b| &b.asset == asset)
            .map(|b| b.available + b.locked)
            .sum()
    }
}
