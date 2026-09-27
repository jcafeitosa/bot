use axum::{extract::State, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::presentation::http::state::ApiState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HttpSeamsMeta {
    pub order_execution_mode: String,
    pub http_admin_auth_enabled: bool,
    pub http_owner_binding_active: bool,
    pub http_agency_binding_active: bool,
    pub bot_runtime_enabled: bool,
    pub live_exchange_wired: bool,
    pub order_reconciliation_pending: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MetaResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub openapi_path: &'static str,
    pub docs_path: &'static str,
    pub http_seams: HttpSeamsMeta,
}

#[utoipa::path(
    get,
    path = "/api/v1/meta",
    tag = "system",
    responses((status = 200, description = "Service metadata", body = MetaResponse))
)]
pub async fn meta(State(state): State<ApiState>) -> Json<MetaResponse> {
    Json(MetaResponse {
        name: "rust-trading-bot",
        version: env!("CARGO_PKG_VERSION"),
        openapi_path: "/openapi.json",
        docs_path: "/docs",
        http_seams: HttpSeamsMeta {
            order_execution_mode: state.order_execution_mode().as_api_label().to_string(),
            http_admin_auth_enabled: state.http_admin_auth_enabled(),
            http_owner_binding_active: state.http_owner_binding_active(),
            http_agency_binding_active: state.http_agency_binding_active(),
            bot_runtime_enabled: state.bot_runtime_status().runtime_enabled,
            live_exchange_wired: state.live_exchange_wired(),
            order_reconciliation_pending: state.order_reconciliation_pending_count_observed().await,
        },
    })
}
