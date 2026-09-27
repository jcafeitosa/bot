use serde::Serialize;
use utoipa::ToSchema;

use crate::core::config::Config;

#[derive(Debug, Serialize, ToSchema)]
pub struct ProvidersStatusResponse {
    pub jev_enabled: bool,
    pub jev_market_regime: bool,
    pub jev_signal_review: bool,
    pub jev_ops_triage: bool,
    pub openai_base_url_configured: bool,
}

pub fn status_from_config(config: &Config) -> ProvidersStatusResponse {
    ProvidersStatusResponse {
        jev_enabled: config.jev.enabled,
        jev_market_regime: config.jev.market_regime,
        jev_signal_review: config.jev.signal_review,
        jev_ops_triage: config.jev.ops_triage,
        openai_base_url_configured: config.providers.openai_base_url.is_some(),
    }
}
