use super::exchange_order_gate::{default_dev_spot_account, gate_order_submit};
use super::execution_port::OrderExecutionPort;
use super::spot_order_submit::submit_spot_order;
use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};

/// Gate 2 spot executor: REST policy gate then in-process recording backend (no network).
#[derive(Debug, Clone, Copy, Default)]
pub struct ExchangeSpotExecutor;

impl OrderExecutionPort for ExchangeSpotExecutor {
    fn execute(&self, request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError> {
        gate_order_submit(&default_dev_spot_account())?;
        submit_spot_order(request).map(|_| ())
    }
}
