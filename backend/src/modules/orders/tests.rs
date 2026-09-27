use super::{submit_order, FailClosedExecutor, OrderSide, OrdersError, SubmitOrderRequest};
use crate::modules::risk::models::RiskLimits;

fn sample_request() -> SubmitOrderRequest<'static> {
    SubmitOrderRequest {
        symbol: "BTC/USDT",
        side: OrderSide::Buy,
        quote_amount: 5.0,
        estimated_daily_loss: 0.0,
        open_positions: 0,
    }
}

fn sample_limits() -> RiskLimits {
    RiskLimits {
        max_order_quote: 10.0,
        max_daily_loss_quote: 20.0,
        max_open_positions: 1,
    }
}

#[test]
fn submit_rejects_quote_over_risk_cap() {
    let request = SubmitOrderRequest {
        symbol: "BTC/USDT",
        side: OrderSide::Buy,
        quote_amount: 50.0,
        estimated_daily_loss: 0.0,
        open_positions: 0,
    };
    let err = submit_order(request, sample_limits(), &FailClosedExecutor).unwrap_err();
    assert!(matches!(err, OrdersError::RiskRejected(_)));
}

#[test]
fn submit_returns_execution_disabled_after_risk_passes() {
    let err = submit_order(sample_request(), sample_limits(), &FailClosedExecutor).unwrap_err();
    assert_eq!(err, OrdersError::ExecutionDisabled);
}
