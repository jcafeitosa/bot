//! Test/double executor for `OrderExecutionPort`. Production HTTP still uses `FailClosedExecutor`.

use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};

use super::OrderExecutionPort;

#[derive(Debug, Clone, Copy, Default)]
pub struct AcceptingExecutor;

impl OrderExecutionPort for AcceptingExecutor {
    fn execute(&self, _request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError> {
        Ok(())
    }
}
