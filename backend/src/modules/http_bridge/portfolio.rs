use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::core::error::BotError;
use crate::modules::orders::{OrderSide, PaperLedgerExecutor};
use crate::modules::portfolio::{paper_snapshot_with_fills, Asset, PaperFillLine, PaperFillSide};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct PaperSnapshotQuery {
    #[param(example = "usdt")]
    pub quote: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaperPositionLine {
    pub symbol: String,
    pub base_asset: String,
    pub quantity: String,
    pub average_entry_price: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaperSnapshotResponse {
    pub quote: String,
    pub as_of_ms: i64,
    pub available: String,
    pub locked: String,
    pub positions: Vec<PaperPositionLine>,
}

pub fn paper_wallet_snapshot(query: PaperSnapshotQuery) -> Result<PaperSnapshotResponse, BotError> {
    let asset =
        Asset::new(query.quote).map_err(|error| BotError::Configuration(error.to_string()))?;
    let fills = PaperLedgerExecutor::recorded_fills()
        .into_iter()
        .map(|fill| PaperFillLine {
            symbol: fill.symbol,
            side: match fill.side {
                OrderSide::Buy => PaperFillSide::Buy,
                OrderSide::Sell => PaperFillSide::Sell,
            },
            quote_amount: fill.quote_amount,
            fill_unit_price: fill.fill_unit_price,
        })
        .collect::<Vec<_>>();
    let snapshot = paper_snapshot_with_fills(&asset, &fills);
    let balance = snapshot
        .balances
        .first()
        .ok_or_else(|| BotError::Configuration("missing balance in paper snapshot".into()))?;
    let positions = snapshot
        .positions
        .iter()
        .map(|position| PaperPositionLine {
            symbol: position.symbol.clone(),
            base_asset: position.base_asset.as_str().to_owned(),
            quantity: position.quantity.to_string(),
            average_entry_price: position.average_entry_price.to_string(),
        })
        .collect();
    Ok(PaperSnapshotResponse {
        quote: asset.as_str().to_owned(),
        as_of_ms: snapshot.as_of_ms,
        available: balance.available.to_string(),
        locked: balance.locked.to_string(),
        positions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::orders::{
        OrderExecutionPort, OrderSide, PaperLedgerExecutor, SubmitOrderRequest,
    };
    use std::sync::Mutex;

    static PORTFOLIO_LEDGER_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_portfolio_ledger_test_lock<F: FnOnce()>(f: F) {
        let _guard = PORTFOLIO_LEDGER_TEST_LOCK
            .lock()
            .expect("portfolio ledger test lock");
        f();
    }

    #[test]
    fn paper_wallet_snapshot_reflects_in_process_ledger() {
        use crate::core::test_env_lock::with_env_test_lock;

        with_portfolio_ledger_test_lock(|| {
            PaperLedgerExecutor::clear_ledger();
            with_env_test_lock(|| {
                std::env::remove_var("BOT_PAPER_FILL_UNIT_PRICE");
                let request = SubmitOrderRequest {
                    symbol: "BTC/USDT",
                    side: OrderSide::Buy,
                    quote_amount: 12.5,
                    estimated_daily_loss: 0.0,
                    open_positions: 0,
                    paper_fill_unit_price: None,
                    client_order_id: None,
                };
                PaperLedgerExecutor.execute(&request).expect("paper fill");
                let response = paper_wallet_snapshot(PaperSnapshotQuery {
                    quote: "usdt".into(),
                })
                .expect("snapshot");
                assert_eq!(response.quote, "USDT");
                assert_eq!(response.available, "987.5");
                assert_eq!(response.locked, "0");
                assert!(response.positions.is_empty());
                assert!(response.as_of_ms > 0);
            });
            PaperLedgerExecutor::clear_ledger();
        });
    }

    #[test]
    fn paper_wallet_snapshot_includes_position_when_fill_unit_price_on_request() {
        with_portfolio_ledger_test_lock(|| {
            PaperLedgerExecutor::clear_ledger();
            let request = SubmitOrderRequest {
                symbol: "BTC/USDT",
                side: OrderSide::Buy,
                quote_amount: 100.0,
                estimated_daily_loss: 0.0,
                open_positions: 0,
                paper_fill_unit_price: Some(50_000.0),
                client_order_id: None,
            };
            PaperLedgerExecutor.execute(&request).expect("paper fill");
            let response = paper_wallet_snapshot(PaperSnapshotQuery {
                quote: "usdt".into(),
            })
            .expect("snapshot");
            assert_eq!(response.positions.len(), 1);
            assert_eq!(response.positions[0].base_asset, "BTC");
            PaperLedgerExecutor::clear_ledger();
        });
    }
}
