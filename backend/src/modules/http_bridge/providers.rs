use serde::Serialize;
use utoipa::ToSchema;

use crate::core::config::Config;
use crate::core::providers::nine_router::resolve_openai_base_url;
use crate::core::providers::nvidia_nim::resolve_nim_base_url;

#[derive(Debug, Serialize, ToSchema)]
pub struct ProvidersStatusResponse {
    pub jev_enabled: bool,
    pub jev_market_regime: bool,
    pub jev_signal_review: bool,
    pub jev_ops_triage: bool,
    pub openai_base_url_configured: bool,
    pub nim_base_url_configured: bool,
    pub resolved_openai_base_url: Option<String>,
    pub resolved_nim_base_url: String,
}

pub fn status_from_config(config: &Config) -> ProvidersStatusResponse {
    ProvidersStatusResponse {
        jev_enabled: config.jev.enabled,
        jev_market_regime: config.jev.market_regime,
        jev_signal_review: config.jev.signal_review,
        jev_ops_triage: config.jev.ops_triage,
        openai_base_url_configured: config.providers.openai_base_url.is_some(),
        nim_base_url_configured: config.providers.nim_base_url.is_some(),
        resolved_openai_base_url: resolve_openai_base_url(&config.providers),
        resolved_nim_base_url: resolve_nim_base_url(&config.providers),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::Config;

    #[test]
    fn status_resolves_default_nim_base_url() {
        let status = status_from_config(&Config::default());
        assert!(status.resolved_nim_base_url.contains("nvidia.com"));
        assert!(!status.nim_base_url_configured);
    }
}
