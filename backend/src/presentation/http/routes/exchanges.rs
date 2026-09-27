use axum::Json;

use crate::modules::http_bridge::exchanges::{
    self, ExchangeCatalogResponse, ExchangeRoutingResponse,
};

#[utoipa::path(
    get,
    path = "/api/v1/exchanges/catalog",
    tag = "exchanges",
    responses((status = 200, description = "Exchange capability catalog", body = ExchangeCatalogResponse))
)]
pub async fn catalog() -> Json<ExchangeCatalogResponse> {
    Json(exchanges::exchange_catalog())
}

#[utoipa::path(
    get,
    path = "/api/v1/exchanges/routing",
    tag = "exchanges",
    responses((status = 200, description = "Market need routing matrix", body = ExchangeRoutingResponse))
)]
pub async fn routing_matrix() -> Json<ExchangeRoutingResponse> {
    Json(exchanges::exchange_routing_matrix())
}
