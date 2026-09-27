use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct MetaResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub openapi_path: &'static str,
    pub docs_path: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/v1/meta",
    tag = "system",
    responses((status = 200, description = "Service metadata", body = MetaResponse))
)]
pub async fn meta() -> Json<MetaResponse> {
    Json(MetaResponse {
        name: "rust-trading-bot",
        version: env!("CARGO_PKG_VERSION"),
        openapi_path: "/openapi.json",
        docs_path: "/docs",
    })
}
