use super::error::OrdersError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct SubmitOrderRequest<'a> {
    pub symbol: &'a str,
    pub side: OrderSide,
    pub quote_amount: f64,
    pub estimated_daily_loss: f64,
    pub open_positions: usize,
}

impl<'a> SubmitOrderRequest<'a> {
    pub fn validate(&self) -> Result<(), OrdersError> {
        if self.symbol.trim().is_empty() {
            return Err(OrdersError::InvalidRequest("symbol is required".into()));
        }
        if !self.quote_amount.is_finite() || self.quote_amount <= 0.0 {
            return Err(OrdersError::InvalidRequest(
                "quote_amount must be finite and positive".into(),
            ));
        }
        if !self.estimated_daily_loss.is_finite() || self.estimated_daily_loss < 0.0 {
            return Err(OrdersError::InvalidRequest(
                "estimated_daily_loss must be finite and non-negative".into(),
            ));
        }
        Ok(())
    }

    pub fn is_close(&self) -> bool {
        matches!(self.side, OrderSide::Sell)
    }
}
