//! In-process test double for Gate 2 order execution (no network; not a live exchange adapter).

use std::cell::Cell;
use std::sync::Mutex;

use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};

use super::OrderExecutionPort;

#[derive(Debug, Default)]
pub struct RecordingExecutor {
    calls: Mutex<Cell<u32>>,
    last_client_order_id: Mutex<Option<String>>,
}

impl RecordingExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn call_count(&self) -> u32 {
        self.calls.lock().expect("lock").get()
    }

    pub fn last_client_order_id(&self) -> Option<String> {
        self.last_client_order_id.lock().expect("lock").clone()
    }
}

impl OrderExecutionPort for RecordingExecutor {
    fn execute(&self, request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError> {
        let guard = self.calls.lock().expect("lock");
        guard.set(guard.get() + 1);
        *self.last_client_order_id.lock().expect("lock") =
            request.client_order_id.map(str::to_string);
        Ok(())
    }
}
