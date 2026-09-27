use std::env;

use super::super::{Credentials, Environment};

pub fn binance_testnet_api_key() -> Option<String> {
    env_nonempty("BINANCE_TESTNET_API_KEY")
}

pub fn binance_testnet_secret() -> Option<String> {
    env_nonempty("BINANCE_TESTNET_SECRET")
}

pub fn binance_testnet_credentials_configured() -> bool {
    binance_testnet_api_key().is_some() && binance_testnet_secret().is_some()
}

pub fn credentials_for_environment(environment: Environment) -> Credentials {
    let prefix = match environment {
        Environment::Dev => "BINANCE_TESTNET",
        Environment::Prod => "BINANCE_PROD",
    };
    let api_key = env::var(format!("{prefix}_API_KEY")).ok();
    let secret = env::var(format!("{prefix}_SECRET")).ok();
    Credentials { api_key, secret }
}

pub fn redact_known_testnet_credentials(message: &str) -> String {
    const REDACTED: &str = "<redacted>";
    let mut out = message.to_string();
    for value in [binance_testnet_api_key(), binance_testnet_secret()]
        .into_iter()
        .flatten()
    {
        out = out.replace(&value, REDACTED);
    }
    out
}

fn env_nonempty(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|v| !v.is_empty())
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
            std::env::remove_var("BINANCE_TESTNET_API_KEY");
            std::env::remove_var("BINANCE_TESTNET_SECRET");
        });
    }
}
