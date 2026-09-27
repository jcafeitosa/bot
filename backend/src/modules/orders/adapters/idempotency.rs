use std::collections::HashSet;
use std::sync::Mutex;

/// Sync dedupe seam for HTTP order submit (`client_order_id`).
pub trait OrderIdempotencyStore: Send + Sync {
    fn is_completed(&self, key: &str) -> bool;
    fn record_completed(&self, key: &str);
}

/// In-process dedupe for HTTP order submit replays (Gate 2 seam; not durable across restarts).
#[derive(Debug, Default)]
pub struct InMemoryOrderIdempotencyStore {
    completed: Mutex<HashSet<String>>,
}

impl InMemoryOrderIdempotencyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OrderIdempotencyStore for InMemoryOrderIdempotencyStore {
    fn is_completed(&self, key: &str) -> bool {
        self.completed
            .lock()
            .expect("order idempotency lock")
            .contains(key)
    }

    fn record_completed(&self, key: &str) {
        self.completed
            .lock()
            .expect("order idempotency lock")
            .insert(key.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::{InMemoryOrderIdempotencyStore, OrderIdempotencyStore};

    #[test]
    fn record_and_detect_completed_keys() {
        let store = InMemoryOrderIdempotencyStore::new();
        assert!(!store.is_completed("k1"));
        store.record_completed("k1");
        assert!(store.is_completed("k1"));
    }
}
