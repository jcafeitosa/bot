pub mod controllers;
pub mod models;

pub use controllers::paper_snapshot;
pub use models::Asset;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rust_decimal::Decimal;

    use super::models::{Asset, PortfolioSnapshot, WalletBalance};

    #[test]
    fn combines_available_and_locked_balance_exactly() {
        let usdt = Asset::new("usdt").unwrap();
        let snapshot = PortfolioSnapshot {
            balances: vec![WalletBalance {
                asset: usdt.clone(),
                available: Decimal::new(125, 2),
                locked: Decimal::new(25, 2),
            }],
            positions: vec![],
            realized_pnl_by_quote: BTreeMap::new(),
            as_of_ms: 10,
        };
        assert_eq!(snapshot.balance(&usdt), Decimal::new(150, 2));
        assert!(snapshot.validate().is_ok());
    }
}
