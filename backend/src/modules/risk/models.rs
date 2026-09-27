#[derive(Debug, Clone, Copy)]
pub struct RiskLimits {
    pub max_order_quote: f64,
    pub max_daily_loss_quote: f64,
    pub max_open_positions: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct OrderIntent {
    pub quote_amount: f64,
    pub estimated_daily_loss: f64,
    pub open_positions: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionContext {
    pub open_positions: usize,
    pub estimated_daily_loss_quote: f64,
}
