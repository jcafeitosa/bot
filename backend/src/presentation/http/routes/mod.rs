pub mod agents;
pub mod application;
pub mod backtest;
pub mod bots;
pub mod config;
pub mod exchanges;
pub mod health;
pub mod meta;
pub mod monitor;
pub mod orders;
pub mod portfolio;
pub mod providers;
pub mod risk;
pub mod strategy;

use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::http::state::ApiState;

pub fn v1_routes() -> Router<ApiState> {
    Router::new()
        .route("/meta", get(meta::meta))
        .route("/application/signals", get(application::list_signals))
        .route("/config/active", get(config::config_active))
        .route("/config/snapshot", get(config::config_snapshot))
        .route("/providers/status", get(providers::provider_status))
        .route("/exchanges/catalog", get(exchanges::catalog))
        .route("/exchanges/routing", get(exchanges::routing_matrix))
        .route("/agents/audit", get(agents::audit_log))
        .route(
            "/agents",
            get(agents::list_agents).post(agents::register_agent),
        )
        .route("/agents/{agent_id}/pause", post(agents::pause_agent))
        .route("/agents/{agent_id}/resume", post(agents::resume_agent))
        .route("/agents/{agent_id}/retire", post(agents::retire_agent))
        .route("/agents/{agent_id}/advisory", post(agents::run_advisory))
        .route("/agents/{agent_id}", get(agents::get_agent))
        .route("/risk/profile-limits", post(risk::compute_profile_limits))
        .route("/risk/validate-intent", post(risk::validate_order_intent))
        .route("/risk/gate-signal", post(risk::gate_signal))
        .route("/strategy/periods", get(strategy::sma_periods))
        .route("/strategy/evaluate-sma", post(strategy::evaluate_sma))
        .route("/portfolio/paper-snapshot", get(portfolio::paper_wallet))
        .route("/bots/catalog", get(bots::bot_catalog))
        .route("/bots/catalog/persist", post(bots::bot_catalog_persist))
        .route("/bots/ranking", post(bots::bot_ranking))
        .route("/backtest/sma-crossover", post(backtest::run_sma_backtest))
        .route("/orders/submit", post(orders::submit_order))
        .route("/monitor/snapshot", get(monitor::snapshot))
        .route("/monitor/commands", post(monitor::post_command))
}

pub fn system_routes() -> Router<ApiState> {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
}
