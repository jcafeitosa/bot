use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::modules::orders::models::{
    OrderReconciliationLedger, OrderSide, OrdersError, PendingReconciliationItem,
    ReconciliationState,
};

#[cfg(test)]
static SHARED_RECONCILIATION_TEST_SERIAL: Mutex<()> = Mutex::new(());

static SHARED_LIVE_RECONCILIATION: OnceLock<Arc<InMemoryOrderReconciliationLedger>> =
    OnceLock::new();

#[derive(Debug, Clone)]
struct StoredEntry {
    state: ReconciliationState,
    symbol: String,
    side: OrderSide,
}

#[derive(Debug, Default)]
pub struct InMemoryOrderReconciliationLedger {
    entries: Mutex<HashMap<String, StoredEntry>>,
}

impl InMemoryOrderReconciliationLedger {
    fn with_entries_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut HashMap<String, StoredEntry>) -> R,
    {
        let mut guard = self.entries.lock().expect("reconciliation lock");
        f(&mut guard)
    }

    fn with_entries<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&HashMap<String, StoredEntry>) -> R,
    {
        let guard = self.entries.lock().expect("reconciliation lock");
        f(&guard)
    }

    #[cfg(test)]
    pub fn clear_all_entries_for_test(&self) {
        self.entries.lock().expect("reconciliation lock").clear();
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// HTTP boot: mirror durable rows into process memory (no exchange side effects).
    pub fn seed_entry(&self, client_order_id: &str, state: ReconciliationState) {
        self.seed_hydrated_row(client_order_id, "", OrderSide::Buy, state);
    }

    pub fn seed_hydrated_row(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: OrderSide,
        state: ReconciliationState,
    ) {
        let key = client_order_id.trim();
        if key.is_empty() {
            return;
        }
        self.with_entries_mut(|map| {
            map.insert(
                key.to_string(),
                StoredEntry {
                    state,
                    symbol: symbol.to_string(),
                    side,
                },
            );
        });
    }
}

impl OrderReconciliationLedger for InMemoryOrderReconciliationLedger {
    fn mark_pending(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: OrderSide,
    ) -> Result<(), OrdersError> {
        let key = client_order_id.trim();
        if key.is_empty() || key.len() > 128 {
            return Err(OrdersError::InvalidRequest(
                "client_order_id must be 1..=128 bytes".into(),
            ));
        }
        self.with_entries_mut(|map| {
            if map.contains_key(key) {
                return Err(OrdersError::InvalidRequest(
                    "client_order_id already tracked for reconciliation".into(),
                ));
            }
            map.insert(
                key.to_string(),
                StoredEntry {
                    state: ReconciliationState::Pending,
                    symbol: symbol.to_string(),
                    side,
                },
            );
            Ok(())
        })
    }

    fn confirm_exchange_order(
        &self,
        client_order_id: &str,
        exchange_order_id: &str,
    ) -> Result<ReconciliationState, OrdersError> {
        let key = client_order_id.trim();
        let exchange_id = exchange_order_id.trim();
        if exchange_id.is_empty() {
            return Err(OrdersError::InvalidRequest(
                "exchange_order_id is required".into(),
            ));
        }
        self.with_entries_mut(|map| {
            let pending_meta = map.get(key).and_then(|entry| {
                if matches!(entry.state, ReconciliationState::Pending) {
                    Some((entry.symbol.clone(), entry.side))
                } else {
                    None
                }
            });
            match (map.get(key).map(|e| e.state.clone()), pending_meta) {
                (Some(ReconciliationState::Pending), Some((symbol, side))) => {
                    let next = ReconciliationState::Reconciled {
                        exchange_order_id: exchange_id.to_string(),
                    };
                    map.insert(
                        key.to_string(),
                        StoredEntry {
                            state: next.clone(),
                            symbol,
                            side,
                        },
                    );
                    Ok(next)
                }
                (Some(ReconciliationState::Reconciled { .. }), _) => Err(
                    OrdersError::InvalidRequest("client_order_id already reconciled".into()),
                ),
                (Some(ReconciliationState::Divergent { .. }), _) => Err(
                    OrdersError::InvalidRequest("client_order_id marked divergent".into()),
                ),
                (None, _) | (_, None) => Err(OrdersError::InvalidRequest(
                    "unknown client_order_id for reconciliation".into(),
                )),
            }
        })
    }

    fn mark_divergent(&self, client_order_id: &str, reason: &str) -> Result<(), OrdersError> {
        let key = client_order_id.trim();
        self.with_entries_mut(|map| {
            let pending_meta = map.get(key).and_then(|entry| {
                if matches!(entry.state, ReconciliationState::Pending) {
                    Some((entry.symbol.clone(), entry.side))
                } else {
                    None
                }
            });
            match pending_meta {
                Some((symbol, side)) => {
                    map.insert(
                        key.to_string(),
                        StoredEntry {
                            state: ReconciliationState::Divergent {
                                reason: reason.to_string(),
                            },
                            symbol,
                            side,
                        },
                    );
                    Ok(())
                }
                None => match map.get(key).map(|e| &e.state) {
                    None => Err(OrdersError::InvalidRequest(
                        "unknown client_order_id for reconciliation".into(),
                    )),
                    _ => Err(OrdersError::InvalidRequest(
                        "reconciliation state cannot become divergent".into(),
                    )),
                },
            }
        })
    }

    fn state(&self, client_order_id: &str) -> Option<ReconciliationState> {
        self.with_entries(|map| {
            map.get(client_order_id.trim())
                .map(|entry| entry.state.clone())
        })
    }

    fn pending_count(&self) -> usize {
        self.with_entries(|map| {
            map.values()
                .filter(|entry| matches!(entry.state, ReconciliationState::Pending))
                .count()
        })
    }

    fn list_pending(&self) -> Vec<PendingReconciliationItem> {
        self.with_entries(|map| {
            map.iter()
                .filter_map(|(client_order_id, entry)| {
                    if !matches!(entry.state, ReconciliationState::Pending) {
                        return None;
                    }
                    Some(PendingReconciliationItem {
                        client_order_id: client_order_id.clone(),
                        symbol: entry.symbol.clone(),
                        side: entry.side,
                    })
                })
                .collect()
        })
    }
}

/// Process-wide ledger for HTTP `ApiState` and monitor testnet submits (same reconciliation view).
pub fn shared_live_order_reconciliation_ledger() -> Arc<InMemoryOrderReconciliationLedger> {
    SHARED_LIVE_RECONCILIATION
        .get_or_init(|| Arc::new(InMemoryOrderReconciliationLedger::new()))
        .clone()
}

/// Test-only: serializes ledger users and clears shared state.
/// Do not hold the returned guard across `.await` unless tests run with `--test-threads=1` (see `scripts/verify-backend-gates.sh`).
/// When combined with [`crate::core::test_env_lock::EnvTestGuard`], acquire env **before** this lock.
#[cfg(test)]
pub fn lock_shared_live_order_reconciliation_ledger_for_test() -> std::sync::MutexGuard<'static, ()>
{
    let guard = SHARED_RECONCILIATION_TEST_SERIAL
        .lock()
        .expect("shared reconciliation test serial");
    shared_live_order_reconciliation_ledger().clear_all_entries_for_test();
    guard
}

#[cfg(test)]
pub fn clear_shared_live_order_reconciliation_ledger_for_test() {
    let _guard = lock_shared_live_order_reconciliation_ledger_for_test();
}
