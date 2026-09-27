//! Non-secret env probes for exchange credentials (values never logged).

/// Both `BINANCE_TESTNET_API_KEY` and `BINANCE_TESTNET_SECRET` set and non-empty.
pub fn binance_testnet_credentials_configured() -> bool {
    let key = std::env::var("BINANCE_TESTNET_API_KEY").ok();
    let secret = std::env::var("BINANCE_TESTNET_SECRET").ok();
    match (key, secret) {
        (Some(k), Some(s)) => !k.trim().is_empty() && !s.trim().is_empty(),
        _ => false,
    }
}

pub fn dev_spot_order_submit_testnet_seam_env_set() -> bool {
    std::env::var("BOT_ORDERS_EXCHANGE_SUBMIT")
        .map(|raw| raw.trim().eq_ignore_ascii_case("testnet"))
        .unwrap_or(false)
}

/// REST policy may allow `OrderSubmit` when testnet submit is selected and credentials exist.
pub fn dev_spot_order_submit_testnet_seam_enabled() -> bool {
    dev_spot_order_submit_testnet_seam_env_set() && binance_testnet_credentials_configured()
}

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

            std::env::set_var("BINANCE_TESTNET_API_KEY", "  ");
            assert!(!binance_testnet_credentials_configured());

            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
        });
    }
}
