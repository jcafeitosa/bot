pub mod application;
pub mod backtest;
pub mod exchanges;
pub mod health;
pub mod meta;
pub mod monitor;
pub mod portfolio;
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
        .route("/exchanges/catalog", get(exchanges::catalog))
        .route("/exchanges/routing", get(exchanges::routing_matrix))
        .route("/risk/profile-limits", post(risk::compute_profile_limits))
        .route("/risk/validate-intent", post(risk::validate_order_intent))
        .route("/strategy/periods", get(strategy::sma_periods))
        .route("/portfolio/paper-snapshot", get(portfolio::paper_wallet))
        .route("/backtest/sma-crossover", post(backtest::run_sma_backtest))
        .route("/monitor/snapshot", get(monitor::snapshot))
        .route("/monitor/commands", post(monitor::post_command))
}

pub fn system_routes() -> Router<ApiState> {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
}
