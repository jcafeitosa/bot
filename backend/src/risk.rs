use crate::{
    config::{RiskProfile, RunMode},
    error::{BotError, BotResult},
    strategy::Signal,
};

#[derive(Debug, Clone, Copy)]
pub struct RiskLimits {
    pub max_order_quote: f64,
    pub max_daily_loss_quote: f64,
    pub max_open_positions: usize,
}

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

#[derive(Debug, Clone, Copy)]
pub struct OrderIntent {
    pub quote_amount: f64,
    pub estimated_daily_loss: f64,
    pub open_positions: usize,
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionContext {
    pub open_positions: usize,
    pub estimated_daily_loss_quote: f64,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::Signal;
    #[test]
    fn all_profiles_remain_at_or_below_base_cap() {
        let base = RiskLimits {
            max_order_quote: 100.,
            max_daily_loss_quote: 50.,
            max_open_positions: 1,
        };
        for p in [
            RiskProfile::Conservative,
            RiskProfile::Moderate,
            RiskProfile::Aggressive,
            RiskProfile::Auto,
        ] {
            assert!(profile_limits(p, base).max_order_quote <= base.max_order_quote);
        }
    }
    #[test]
    fn rejects_non_finite_limits() {
        let limits = RiskLimits {
            max_order_quote: f64::NAN,
            max_daily_loss_quote: 20.,
            max_open_positions: 1,
        };
        assert!(validate_intent(
            OrderIntent {
                quote_amount: 5.,
                estimated_daily_loss: 0.,
                open_positions: 0
            },
            limits,
            false,
        )
        .is_err());
    }
    #[test]
    fn gate_signal_updates_paper_ledger() {
        let limits = RiskLimits {
            max_order_quote: 10.,
            max_daily_loss_quote: 20.,
            max_open_positions: 1,
        };
        let mut ctx = ExecutionContext::default();
        gate_signal(Signal::Buy, limits, RunMode::Paper, &mut ctx).unwrap();
        assert_eq!(ctx.open_positions, 1);
        gate_signal(Signal::Sell, limits, RunMode::Paper, &mut ctx).unwrap();
        assert_eq!(ctx.open_positions, 0);
    }
    #[test]
    fn rejects_order_over_limit() {
        let limits = RiskLimits {
            max_order_quote: 10.,
            max_daily_loss_quote: 20.,
            max_open_positions: 1,
        };
        assert!(validate_intent(
            OrderIntent {
                quote_amount: 11.,
                estimated_daily_loss: 0.,
                open_positions: 0
            },
            limits,
            false,
        )
        .is_err());
    }
}
