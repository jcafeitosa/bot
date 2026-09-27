use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};

pub trait OrderExecutionPort {
    fn execute(&self, request: &SubmitOrderRequest<'_>) -> Result<(), OrdersError>;
}
