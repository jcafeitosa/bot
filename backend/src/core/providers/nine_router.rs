#![allow(dead_code)]

use crate::core::config::ProviderConfig;

pub fn openai_base_url_from_env() -> Option<String> {
    crate::core::config::providers::openai_base_url_from_env()
}

pub fn resolve_openai_base_url(toml: &ProviderConfig) -> Option<String> {
    crate::core::config::providers::resolve_openai_base_url(toml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ProviderConfig;

    #[test]
    fn env_precedes_toml_for_base_url() {
        crate::core::test_env_lock::with_env_test_lock(|| {
            std::env::set_var("NINE_ROUTER_BASE_URL", "https://router.example");
            let resolved = resolve_openai_base_url(&ProviderConfig {
                openai_base_url: Some("https://toml.example".into()),
                nim_base_url: None,
            });
            std::env::remove_var("NINE_ROUTER_BASE_URL");
            assert_eq!(resolved.as_deref(), Some("https://router.example"));
        });
    }
}
