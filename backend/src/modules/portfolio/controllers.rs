use rust_decimal::Decimal;
use std::collections::BTreeMap;

use super::models::{Asset, PortfolioSnapshot, WalletBalance};

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
