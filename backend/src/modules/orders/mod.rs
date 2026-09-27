//! Order submission seam — risk validation then fail-closed execution port.
//!
//! Live trading remains disabled; see `modules/exchanges/rest::authorize_rest_use`.

#![allow(dead_code)] // public foundation seam; live wiring intentionally absent
#![allow(unused_imports)]

pub mod adapters;
pub mod controllers;
pub mod models;

pub use adapters::{
    clear_live_reconciliation_pg_mirror, live_exchange_submit_backend,
    live_exchange_submit_backend_enabled, recording_bind_client_exchange,
    register_live_reconciliation_pg_mirror, shared_live_order_reconciliation_ledger,
    take_last_spot_submit_ack, try_mirror_reconciliation_upsert, AcceptingExecutor,
    ExchangeSpotExecutor, FailClosedExecutor, InMemoryOrderIdempotencyStore,
    InMemoryOrderReconciliationLedger, LiveExchangeSpotOrderReconciliationQuery,
    LiveExchangeSubmitBackend, OrderExecutionPort, OrderIdempotencyStore, PaperFill,
    PaperLedgerExecutor, PgOrderIdempotencyStore, PgOrderReconciliationStore, RecordingExecutor,
    RecordingSpotOrderReconciliationQuery, RecordingSpotOrderSubmitPort, RemoteOrderObservation,
    ReservedLiveExchangeExecutor, SpotOrderReconciliationQuery, SpotOrderSubmitAck,
};
#[cfg(test)]
pub use adapters::{
    clear_shared_live_order_reconciliation_ledger_for_test,
    lock_shared_live_order_reconciliation_ledger_for_test,
};
pub use controllers::{run_reconciliation_poll_once, submit_order, ReconciliationPollSummary};
pub use models::{
    OrderReconciliationLedger, OrderSide, OrdersError, PendingReconciliationItem,
    ReconciliationState, SubmitOrderRequest,
};

#[cfg(test)]
mod tests;
