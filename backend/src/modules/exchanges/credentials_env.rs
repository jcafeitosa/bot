//! Non-secret env probes (re-export from core::config).

pub use crate::core::config::exchanges::binance_testnet_credentials_configured;
pub use crate::core::config::orders::exchange_submit_testnet_enabled as dev_spot_order_submit_testnet_seam_enabled;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn binance_testnet_credentials_requires_both_vars_non_empty() {
        with_env_test_lock(|| {
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
            assert!(!binance_testnet_credentials_configured());
            std::env::set_var("BINANCE_TESTNET_API_KEY", "k");
            assert!(!binance_testnet_credentials_configured());
            std::env::set_var("BINANCE_TESTNET_SECRET", "s");
            assert!(binance_testnet_credentials_configured());
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
        });
    }
}
