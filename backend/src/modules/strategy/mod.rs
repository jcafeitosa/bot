pub mod controllers;
pub mod models;

pub use crate::modules::application_contracts::Signal;
pub use controllers::evaluate;
#[allow(unused_imports)]
pub use controllers::periods_for_mode;
pub use models::StrategySnapshot;

#[cfg(test)]
mod tests {
    use super::*;
    use mantis_ta::types::Candle;
    fn bars(closes: &[f64]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(i, close)| Candle {
                timestamp: i as i64 * 60_000,
                open: *close,
                high: *close,
                low: *close,
                close: *close,
                volume: 1.0,
            })
            .collect()
    }
    #[test]
    fn waits_for_indicator_warmup() {
        let b = bars(&[1., 2., 3., 4.]);
        assert_eq!(evaluate(&b, 3, 5).signal, Signal::Warmup);
    }
    #[test]
    fn detects_fast_sma_crossing_above_slow_sma() {
        let b = bars(&[5., 5., 5., 5., 1., 10.]);
        assert_eq!(evaluate(&b, 2, 4).signal, Signal::Buy);
    }
    #[test]
    fn detects_fast_sma_crossing_below_slow_sma() {
        let b = bars(&[10., 10., 10., 10., 10., 1.]);
        assert_eq!(evaluate(&b, 2, 4).signal, Signal::Sell);
    }
    #[test]
    fn holds_when_both_averages_are_on_the_same_side() {
        let b = bars(&[
            10., 9., 8., 7., 6., 5., 4., 3., 2., 1., 2., 4., 8., 12., 16.,
        ]);
        assert_eq!(evaluate(&b, 2, 4).signal, Signal::Hold);
    }
}
