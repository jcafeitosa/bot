use crate::core::config::OperationMode;

#[allow(dead_code)]
pub fn periods_for_mode(mode: OperationMode) -> (usize, usize) {
    mode.sma_period_preset()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::OperationMode;

    #[test]
    fn matches_config_operation_preset() {
        assert_eq!(
            periods_for_mode(OperationMode::SwingTrader),
            OperationMode::SwingTrader.sma_period_preset()
        );
    }
}
