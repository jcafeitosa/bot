use crate::core::config::{RiskProfile, RunMode};
use crate::core::error::{BotError, BotResult};
use crate::modules::application_contracts::Signal;
use crate::modules::risk::models::{ExecutionContext, OrderIntent, RiskLimits};

pub fn profile_limits(profile: RiskProfile, base: RiskLimits) -> RiskLimits {
    let factor = match profile {
        RiskProfile::Conservative => 0.5,
        RiskProfile::Moderate => 1.0,
        // Aggressive can use the full configured cap, never exceed it.
        RiskProfile::Aggressive => 1.0,
        // Auto can adapt down but never exceed the configured base cap.
        RiskProfile::Auto => 0.75,
    };
    RiskLimits {
        max_order_quote: base.max_order_quote * factor,
        max_daily_loss_quote: base.max_daily_loss_quote,
        max_open_positions: base.max_open_positions,
    }
}

pub fn validate_intent(intent: OrderIntent, limits: RiskLimits, is_close: bool) -> BotResult<()> {
    if !limits.max_order_quote.is_finite()
        || limits.max_order_quote <= 0.0
        || !limits.max_daily_loss_quote.is_finite()
        || limits.max_daily_loss_quote <= 0.0
        || limits.max_open_positions == 0
    {
        return Err(BotError::RiskRejected(
            "configured risk limits are invalid".into(),
        ));
    }
    if !intent.quote_amount.is_finite()
        || intent.quote_amount <= 0.0
        || intent.quote_amount > limits.max_order_quote
    {
        return Err(BotError::RiskRejected(
            "quote size is invalid or exceeds the configured cap".into(),
        ));
    }
    if !intent.estimated_daily_loss.is_finite()
        || intent.estimated_daily_loss < 0.0
        || intent.estimated_daily_loss > limits.max_daily_loss_quote
    {
        return Err(BotError::RiskRejected(
            "daily-loss limit would be exceeded".into(),
        ));
    }
    if !is_close && intent.open_positions >= limits.max_open_positions {
        return Err(BotError::RiskRejected(
            "maximum open positions reached".into(),
        ));
    }
    Ok(())
}

/// Validates a hypothetical order against limits; updates paper ledger when `run_mode` is paper.
pub fn gate_signal(
    signal: Signal,
    limits: RiskLimits,
    run_mode: RunMode,
    ctx: &mut ExecutionContext,
) -> BotResult<()> {
    if !matches!(signal, Signal::Buy | Signal::Sell) {
        return Ok(());
    }
    let intent = OrderIntent {
        quote_amount: limits.max_order_quote,
        estimated_daily_loss: ctx.estimated_daily_loss_quote,
        open_positions: ctx.open_positions,
    };
    validate_intent(intent, limits, signal == Signal::Sell)?;
    if run_mode != RunMode::Paper {
        return Ok(());
    }
    match signal {
        Signal::Buy if ctx.open_positions == 0 => ctx.open_positions = 1,
        Signal::Sell if ctx.open_positions > 0 => ctx.open_positions = 0,
        _ => {}
    }
    Ok(())
}
