use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::modules::http_bridge::risk::RiskLimitsBody;
use crate::modules::orders::{
    submit_order, FailClosedExecutor, OrderSide, OrdersError, SubmitOrderRequest,
};
use crate::modules::risk::models::RiskLimits;

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderSideBody {
    Buy,
    Sell,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitOrderHttpRequest {
    pub symbol: String,
    pub side: OrderSideBody,
    pub quote_amount: f64,
    pub estimated_daily_loss: f64,
    pub open_positions: usize,
    pub limits: RiskLimitsBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitOrderResponse {
    pub accepted: bool,
}

pub fn submit_order_http(body: SubmitOrderHttpRequest) -> Result<SubmitOrderResponse, OrdersError> {
    let SubmitOrderHttpRequest {
        symbol,
        side,
        quote_amount,
        estimated_daily_loss,
        open_positions,
        limits: limits_body,
    } = body;
    let limits: RiskLimits = limits_body.into();
    let request = SubmitOrderRequest {
        symbol: &symbol,
        side: match side {
            OrderSideBody::Buy => OrderSide::Buy,
            OrderSideBody::Sell => OrderSide::Sell,
        },
        quote_amount,
        estimated_daily_loss,
        open_positions,
    };
    submit_order(request, limits, &FailClosedExecutor)?;
    Ok(SubmitOrderResponse { accepted: true })
}
