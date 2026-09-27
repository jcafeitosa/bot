pub mod controllers;
pub mod models;

pub use controllers::{gate_signal, profile_limits};
pub use models::{ExecutionContext, RiskLimits};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{RiskProfile, RunMode};
    use crate::modules::application_contracts::Signal;
    use crate::modules::risk::controllers::validate_intent;
    use crate::modules::risk::models::OrderIntent;
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
