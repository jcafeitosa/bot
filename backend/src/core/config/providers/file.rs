use super::super::load::{env_nonempty, env_override_string};
use super::super::system::SystemConfig;
use super::super::ProviderConfig;
use crate::core::error::{BotError, BotResult};
use crate::core::providers::credentials::{
    lookup_secret, KEY_API_KEY, PROVIDER_NGC, PROVIDER_NVIDIA, PROVIDER_OPENAI, PROVIDER_TYPESAFE,
};

pub fn typesafe_api_key() -> Option<String> {
    lookup_secret(PROVIDER_TYPESAFE, KEY_API_KEY, &["TYPESAFE_API_KEY"])
        .or_else(|| lookup_secret(PROVIDER_OPENAI, KEY_API_KEY, &["OPENAI_API_KEY"]))
}

pub fn resolve_bearer_api_key() -> BotResult<String> {
    typesafe_api_key().ok_or_else(|| {
        BotError::Configuration(
            "jev.enabled=true requires provider API key in PostgreSQL provider_credentials (typesafe/openai api_key) or deprecated bootstrap env TYPESAFE_API_KEY / OPENAI_API_KEY"
                .into(),
        )
    })
}

pub fn typesafe_endpoint() -> String {
    let defaults = SystemConfig::active();
    env_override_string("TYPESAFE_ENDPOINT", &defaults.providers.typesafe_endpoint)
}

pub fn typesafe_model() -> String {
    env_override_string("TYPESAFE_MODEL", "oc/jev-1.13-free")
}

pub fn openai_base_url_from_env() -> Option<String> {
    if let Some(url) =
        env_nonempty("NINE_ROUTER_BASE_URL").or_else(|| env_nonempty("OPENAI_BASE_URL"))
    {
        return Some(url);
    }
    let defaults = SystemConfig::active();
    let url = defaults.providers.openai_base_url.trim();
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

pub fn resolve_openai_base_url(toml: &ProviderConfig) -> Option<String> {
    openai_base_url_from_env().or_else(|| toml.openai_base_url.clone())
}

pub fn nim_base_url_from_env() -> Option<String> {
    if let Some(url) = env_nonempty("NVIDIA_NIM_BASE_URL") {
        return Some(url);
    }
    let defaults = SystemConfig::active();
    let url = defaults.providers.nim_base_url.trim();
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

pub fn resolve_nvidia_api_key() -> BotResult<String> {
    lookup_secret(PROVIDER_NVIDIA, KEY_API_KEY, &["NVIDIA_API_KEY"])
        .or_else(|| lookup_secret(PROVIDER_NGC, KEY_API_KEY, &["NGC_API_KEY"]))
        .ok_or_else(|| {
            BotError::Configuration(
                "NVIDIA NIM requires provider API key in PostgreSQL provider_credentials (nvidia/ngc api_key) or deprecated bootstrap env NVIDIA_API_KEY / NGC_API_KEY"
                    .into(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_precedes_toml_for_openai_base_url() {
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

    #[test]
    fn typesafe_model_defaults_without_env() {
        crate::core::test_env_lock::with_env_test_lock(|| {
            std::env::remove_var("TYPESAFE_MODEL");
            assert_eq!(typesafe_model(), "oc/jev-1.13-free");
        });
    }

    #[test]
    fn typesafe_model_env_override() {
        crate::core::test_env_lock::with_env_test_lock(|| {
            std::env::set_var("TYPESAFE_MODEL", "oc/custom-model");
            assert_eq!(typesafe_model(), "oc/custom-model");
            std::env::remove_var("TYPESAFE_MODEL");
        });
    }
}
