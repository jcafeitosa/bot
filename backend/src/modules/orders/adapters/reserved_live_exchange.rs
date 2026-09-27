use super::exchange_order_gate::{default_dev_spot_account, gate_order_submit};
use super::execution_port::OrderExecutionPort;
use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};

/// Placeholder port for Gate 2 exchange wiring — enforces `authorize_rest_use` then fails until adapter lands.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReservedLiveExchangeExecutor;

impl OrderExecutionPort for ReservedLiveExchangeExecutor {
    fn execute(&self, _request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError> {
        gate_order_submit(&default_dev_spot_account())?;
        Err(OrdersError::LiveExchangeNotWired)
    }
}
