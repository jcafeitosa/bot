//! In-process paper ledger: records fills after risk without exchange REST.

use std::sync::Mutex;

use crate::modules::orders::models::{OrderSide, OrdersError, SubmitOrderRequest};

use super::OrderExecutionPort;

#[derive(Debug, Clone, PartialEq)]
pub struct PaperFill {
    pub symbol: String,
    pub side: OrderSide,
    pub quote_amount: f64,
    pub fill_unit_price: Option<f64>,
}

fn paper_fill_unit_price_from_env() -> Option<f64> {
    crate::core::config::paper_fill_unit_price()
}

static LEDGER: Mutex<Vec<PaperFill>> = Mutex::new(Vec::new());

#[cfg(test)]
static PAPER_LEDGER_TEST_SERIAL: Mutex<()> = Mutex::new(());

fn with_ledger_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vec<PaperFill>) -> R,
{
    #[cfg(test)]
    let _serial = PAPER_LEDGER_TEST_SERIAL
        .lock()
        .expect("paper ledger test serial lock");
    let mut guard = LEDGER.lock().expect("paper ledger lock");
    f(&mut guard)
}

fn with_ledger<F, R>(f: F) -> R
where
    F: FnOnce(&Vec<PaperFill>) -> R,
{
    #[cfg(test)]
    let _serial = PAPER_LEDGER_TEST_SERIAL
        .lock()
        .expect("paper ledger test serial lock");
    let guard = LEDGER.lock().expect("paper ledger lock");
    f(&guard)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PaperLedgerExecutor;

impl PaperLedgerExecutor {
    pub fn recorded_fills() -> Vec<PaperFill> {
        with_ledger(|ledger| ledger.clone())
    }

    pub fn clear_ledger() {
        with_ledger_mut(|ledger| ledger.clear());
    }
}

impl OrderExecutionPort for PaperLedgerExecutor {
    fn execute(&self, request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError> {
        with_ledger_mut(|ledger| {
            ledger.push(PaperFill {
                symbol: request.symbol.to_string(),
                side: request.side,
                quote_amount: request.quote_amount,
                fill_unit_price: request
                    .paper_fill_unit_price
                    .or_else(paper_fill_unit_price_from_env),
            });
        });
        Ok(())
    }
}
