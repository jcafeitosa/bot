use crate::modules::orders::adapters::OrderExecutionPort;
use crate::modules::orders::models::{OrdersError, SubmitOrderRequest};
use crate::modules::risk::controllers::validate_intent;
use crate::modules::risk::models::{OrderIntent, RiskLimits};

pub fn submit_order(
    request: SubmitOrderRequest<'_>,
    limits: RiskLimits,
    executor: &dyn OrderExecutionPort,
) -> Result<(), OrdersError> {
    request.validate()?;
    let intent = OrderIntent {
        quote_amount: request.quote_amount,
        estimated_daily_loss: request.estimated_daily_loss,
        open_positions: request.open_positions,
    };
    validate_intent(intent, limits, request.is_close())
        .map_err(|error| OrdersError::RiskRejected(error.to_string()))?;
    executor.execute(&request)
}
