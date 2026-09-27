use super::execution_port::OrderExecutionPort;
use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};

#[derive(Debug, Clone, Copy, Default)]
pub struct FailClosedExecutor;

impl OrderExecutionPort for FailClosedExecutor {
    fn execute(&self, _request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError> {
        Err(OrdersError::ExecutionDisabled)
    }
}
