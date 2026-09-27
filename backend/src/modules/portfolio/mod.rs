pub mod controllers;
pub mod models;

pub use controllers::{paper_snapshot, paper_snapshot_with_fills};
pub use models::{Asset, PaperFillLine, PaperFillSide};

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rust_decimal::Decimal;

    use super::controllers::paper_initial_quote_balance;
    use super::models::{
        Asset, PaperFillLine, PaperFillSide, PortfolioError, PortfolioSnapshot, WalletBalance,
    };
    use super::{paper_snapshot, paper_snapshot_with_fills};

    #[test]
    fn rejects_invalid_asset_code() {
        assert_eq!(Asset::new("").unwrap_err(), PortfolioError::InvalidAsset);
        assert_eq!(
            Asset::new("btc/usdt").unwrap_err(),
            PortfolioError::InvalidAsset
        );
    }

    #[test]
    fn paper_snapshot_starts_with_initial_quote_balance() {
        let usdt = Asset::new("usdt").unwrap();
        let snapshot = paper_snapshot(&usdt);
        assert_eq!(
            snapshot.balances[0].available,
            paper_initial_quote_balance()
        );
        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn paper_snapshot_applies_ledger_buys_and_sells() {
        let usdt = Asset::new("usdt").unwrap();
        let fills = vec![
            PaperFillLine {
                symbol: "BTC/USDT".into(),
                side: PaperFillSide::Buy,
                quote_amount: 5.0,
                fill_unit_price: None,
            },
            PaperFillLine {
                symbol: "ETH/USDT".into(),
                side: PaperFillSide::Sell,
                quote_amount: 2.0,
                fill_unit_price: None,
            },
            PaperFillLine {
                symbol: "BTC/EUR".into(),
                side: PaperFillSide::Buy,
                quote_amount: 100.0,
                fill_unit_price: None,
            },
        ];
        let snapshot = paper_snapshot_with_fills(&usdt, &fills);
        assert_eq!(
            snapshot.balances[0].available,
            paper_initial_quote_balance() - Decimal::new(5, 0) + Decimal::new(2, 0)
        );
        assert!(snapshot.as_of_ms > 0);
    }

    #[test]
    fn paper_snapshot_tracks_base_position_when_fill_price_present() {
        let usdt = Asset::new("usdt").unwrap();
        let fills = vec![PaperFillLine {
            symbol: "BTC/USDT".into(),
            side: PaperFillSide::Buy,
            quote_amount: 100.0,
            fill_unit_price: Some(50_000.0),
        }];
        let snapshot = paper_snapshot_with_fills(&usdt, &fills);
        assert_eq!(snapshot.positions.len(), 1);
        assert_eq!(snapshot.positions[0].symbol, "BTC/USDT");
        assert_eq!(snapshot.positions[0].quantity, Decimal::new(2, 3));
        assert!(snapshot.validate().is_ok());
    }

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
