#![allow(dead_code)] // Env/TOML resolution seam for OpenAI-compatible proxies (9router).

//! OpenAI-compatible deployment targets (e.g. self-hosted **9router** proxies).
//!
//! This repository does not ship 9router-specific API docs. Point `NINE_ROUTER_BASE_URL` or
//! `OPENAI_BASE_URL` at your proxy's OpenAI-compatible root (typically ending before `/v1/...`).
//! Jev advisory continues to use the full URL in `TYPESAFE_ENDPOINT` unless you expose a
//! SystemOne-compatible route on the same host.

use crate::core::config::ProviderConfig;

pub fn openai_base_url_from_env() -> Option<String> {
    std::env::var("NINE_ROUTER_BASE_URL")
        .ok()
        .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
}

pub fn resolve_openai_base_url(toml: &ProviderConfig) -> Option<String> {
    openai_base_url_from_env().or_else(|| toml.openai_base_url.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ProviderConfig;

    #[test]
    fn env_precedes_toml_for_base_url() {
        std::env::set_var("NINE_ROUTER_BASE_URL", "https://router.example");
        let resolved = resolve_openai_base_url(&ProviderConfig {
            openai_base_url: Some("https://toml.example".into()),
        });
        std::env::remove_var("NINE_ROUTER_BASE_URL");
        assert_eq!(resolved.as_deref(), Some("https://router.example"));
    }
}
