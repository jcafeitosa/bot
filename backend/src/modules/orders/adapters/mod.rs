#![allow(dead_code)] // public seam; live wiring intentionally absent

mod accepting_executor;
mod exchange_order_gate;
mod exchange_spot_executor;
mod execution_port;
mod fail_closed;
mod graph_projection;
mod idempotency;
mod in_memory_reconciliation;
mod live_reconciliation_pg_mirror;
mod paper_ledger_executor;
mod pg_idempotency;
mod pg_reconciliation;
mod recording_executor;
mod reserved_live_exchange;
mod spot_order_reconciliation_query;
mod spot_order_submit;

pub use accepting_executor::AcceptingExecutor;
pub use exchange_spot_executor::ExchangeSpotExecutor;
pub use execution_port::OrderExecutionPort;
pub use fail_closed::FailClosedExecutor;
pub use graph_projection::{best_effort_project_order_intent, RedactedOrderSubmitSnapshot};
pub use idempotency::{InMemoryOrderIdempotencyStore, OrderIdempotencyStore};
#[cfg(test)]
pub use in_memory_reconciliation::{
    clear_shared_live_order_reconciliation_ledger_for_test,
    lock_shared_live_order_reconciliation_ledger_for_test,
};
pub use in_memory_reconciliation::{
    shared_live_order_reconciliation_ledger, InMemoryOrderReconciliationLedger,
};
pub use live_reconciliation_pg_mirror::{
    clear_live_reconciliation_pg_mirror, register_live_reconciliation_pg_mirror,
    try_mirror_reconciliation_upsert,
};
#[cfg(test)]
pub use paper_ledger_executor::PaperLedgerTestGuard;
pub use paper_ledger_executor::{PaperFill, PaperLedgerExecutor};
pub use pg_idempotency::PgOrderIdempotencyStore;
pub use pg_reconciliation::PgOrderReconciliationStore;
pub use recording_executor::RecordingExecutor;
pub use reserved_live_exchange::ReservedLiveExchangeExecutor;
#[cfg(test)]
pub use spot_order_reconciliation_query::clear_recording_client_bindings;
pub use spot_order_reconciliation_query::{
    recording_bind_client_exchange, LiveExchangeSpotOrderReconciliationQuery,
    RecordingSpotOrderReconciliationQuery, RemoteOrderObservation, SpotOrderReconciliationQuery,
};
pub use spot_order_submit::{
    live_exchange_submit_backend, live_exchange_submit_backend_enabled, submit_spot_order,
    take_last_spot_submit_ack, LiveExchangeSubmitBackend, RecordingSpotOrderSubmitPort,
    SpotOrderSubmitAck,
};
