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
static PAPER_LEDGER_TEST_SCOPE: Mutex<()> = Mutex::new(());

/// Hold for the whole test (including `.await`) while using the in-process paper ledger.
#[cfg(test)]
pub struct PaperLedgerTestGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl PaperLedgerTestGuard {
    pub fn acquire() -> Self {
        Self {
            _guard: PAPER_LEDGER_TEST_SCOPE
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        }
    }
}

fn with_ledger_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vec<PaperFill>) -> R,
{
    let mut guard = LEDGER.lock().expect("paper ledger lock");
    f(&mut guard)
}

fn with_ledger<F, R>(f: F) -> R
where
    F: FnOnce(&Vec<PaperFill>) -> R,
{
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
