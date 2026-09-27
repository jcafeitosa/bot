use std::env;

use super::super::exchanges::binance_testnet_credentials_configured;
use super::super::system::SystemConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveExchangeSubmitBackend {
    Recording,
    Testnet,
}

pub fn exchange_submit_mode_raw() -> Option<String> {
    if let Some(mode) = env_nonempty("BOT_ORDERS_EXCHANGE_SUBMIT") {
        return Some(mode);
    }
    let mode = SystemConfig::active().orders.exchange_submit.trim();
    if mode.is_empty() {
        None
    } else {
        Some(mode.to_string())
    }
}

fn env_nonempty(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn exchange_submit_recording_enabled() -> bool {
    exchange_submit_mode_raw()
        .map(|raw| raw.eq_ignore_ascii_case("recording"))
        .unwrap_or(false)
}

pub fn exchange_submit_testnet_env_set() -> bool {
    exchange_submit_mode_raw()
        .map(|raw| raw.eq_ignore_ascii_case("testnet"))
        .unwrap_or(false)
}

pub fn exchange_submit_testnet_enabled() -> bool {
    exchange_submit_testnet_env_set() && binance_testnet_credentials_configured()
}

pub fn live_exchange_submit_backend() -> Option<LiveExchangeSubmitBackend> {
    if exchange_submit_recording_enabled() {
        return Some(LiveExchangeSubmitBackend::Recording);
    }
    if exchange_submit_testnet_enabled() {
        return Some(LiveExchangeSubmitBackend::Testnet);
    }
    None
}

pub fn live_exchange_submit_backend_enabled() -> bool {
    live_exchange_submit_backend().is_some()
}

pub fn paper_fill_unit_price() -> Option<f64> {
    if let Some(raw) = env_nonempty("BOT_PAPER_FILL_UNIT_PRICE") {
        let price = raw.parse::<f64>().ok()?;
        return (price.is_finite() && price > 0.0).then_some(price);
    }
    let price = SystemConfig::active().orders.paper_fill_unit_price;
    (price.is_finite() && price > 0.0).then_some(price)
}

pub fn order_reconciliation_poll_interval_secs() -> Option<u64> {
    if let Some(raw) = env_nonempty("BOT_ORDERS_RECONCILIATION_POLL_SECS") {
        let secs = raw.parse::<u64>().ok()?;
        return (secs > 0).then_some(secs);
    }
    let secs = SystemConfig::active().orders.reconciliation_poll_secs;
    (secs > 0).then_some(secs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpOrderExecutionMode {
    Disabled,
    DevAccept,
    Paper,
    LiveExchange,
    LiveExchangeReserved,
}

fn execution_mode_raw() -> String {
    env_nonempty("BOT_ORDERS_EXECUTION")
        .unwrap_or_else(|| SystemConfig::active().orders.execution.clone())
}

pub fn http_order_execution_mode_from_env() -> HttpOrderExecutionMode {
    let raw = execution_mode_raw();
    if raw.trim().eq_ignore_ascii_case("dev_accept") {
        return HttpOrderExecutionMode::DevAccept;
    }
    if raw.trim().eq_ignore_ascii_case("paper") {
        return HttpOrderExecutionMode::Paper;
    }
    if raw.trim().eq_ignore_ascii_case("live_exchange") {
        return if live_exchange_submit_backend_enabled() {
            HttpOrderExecutionMode::LiveExchange
        } else {
            HttpOrderExecutionMode::LiveExchangeReserved
        };
    }
    if !raw.trim().is_empty() && !raw.trim().eq_ignore_ascii_case("disabled") {
        tracing::warn!(
            target: "api",
            value = raw.trim(),
            "unknown BOT_ORDERS_EXECUTION; using fail-closed disabled"
        );
    }
    HttpOrderExecutionMode::Disabled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_env_lock::with_env_test_lock;

    #[test]
    fn reconciliation_poll_env_overrides_toml() {
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_RECONCILIATION_POLL_SECS", "30");
            assert_eq!(order_reconciliation_poll_interval_secs(), Some(30));
            std::env::remove_var("BOT_ORDERS_RECONCILIATION_POLL_SECS");
        });
    }
}
