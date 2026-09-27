use crate::modules::orders::adapters::{RemoteOrderObservation, SpotOrderReconciliationQuery};
use crate::modules::orders::models::{OrderReconciliationLedger, OrdersError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconciliationPollSummary {
    pub confirmed: usize,
    pub divergent: usize,
    pub unchanged: usize,
}

pub fn run_reconciliation_poll_once(
    ledger: &dyn OrderReconciliationLedger,
    query: &dyn SpotOrderReconciliationQuery,
) -> Result<ReconciliationPollSummary, OrdersError> {
    let mut summary = ReconciliationPollSummary::default();
    for item in ledger.list_pending() {
        let observation = query.observe(&item.client_order_id, &item.symbol, item.side)?;
        match observation {
            RemoteOrderObservation::Confirmed { exchange_order_id } => {
                ledger.confirm_exchange_order(&item.client_order_id, &exchange_order_id)?;
                summary.confirmed += 1;
            }
            RemoteOrderObservation::Divergent { reason } => {
                ledger.mark_divergent(&item.client_order_id, &reason)?;
                summary.divergent += 1;
            }
            RemoteOrderObservation::StillPending => {
                summary.unchanged += 1;
            }
        }
    }
    Ok(summary)
}
