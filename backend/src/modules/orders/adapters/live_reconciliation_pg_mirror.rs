//! Best-effort PostgreSQL mirror for monitor + shared in-memory reconciliation (sync call sites).

use std::sync::{Arc, LazyLock, Mutex};

use super::PgOrderReconciliationStore;
use crate::modules::orders::models::{OrderSide, OrdersError, ReconciliationState};

static PG_MIRROR: Mutex<Option<Arc<PgOrderReconciliationStore>>> = Mutex::new(None);

static MIRROR_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("reconciliation pg mirror runtime")
});

pub fn register_live_reconciliation_pg_mirror(store: Arc<PgOrderReconciliationStore>) {
    *PG_MIRROR.lock().expect("reconciliation pg mirror lock") = Some(store);
}

pub fn clear_live_reconciliation_pg_mirror() {
    *PG_MIRROR.lock().expect("reconciliation pg mirror lock") = None;
}

/// Upserts the row when HTTP boot registered a PG store (no-op otherwise).
pub fn try_mirror_reconciliation_upsert(
    client_order_id: &str,
    symbol: &str,
    side: OrderSide,
    state: &ReconciliationState,
) -> Result<(), OrdersError> {
    let pg = PG_MIRROR
        .lock()
        .expect("reconciliation pg mirror lock")
        .clone();
    if let Some(store) = pg {
        MIRROR_RUNTIME.block_on(store.upsert_state(client_order_id, symbol, side, state))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_without_register_is_noop() {
        clear_live_reconciliation_pg_mirror();
        try_mirror_reconciliation_upsert(
            "cid",
            "BTC/USDT",
            OrderSide::Buy,
            &ReconciliationState::Pending,
        )
        .expect("noop");
    }
}
