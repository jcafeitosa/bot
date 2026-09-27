use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};

use crate::presentation::http::{error::ApiError, state::ApiState};

fn provider_credentials_admin_not_implemented() -> ApiError {
    ApiError::with_code(
        StatusCode::NOT_IMPLEMENTED,
        "provider_credentials_admin_not_implemented",
        "HTTP CRUD for provider_credentials is not implemented; seed via SQL or ops rotation (see provider-credentials-db-sdd.md)",
    )
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/provider-credentials",
    tag = "admin",
    responses(
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 501, description = "Admin CRUD not implemented (fail-closed)", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn list_provider_credentials(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<(), ApiError> {
    state.require_http_admin(&headers)?;
    Err(provider_credentials_admin_not_implemented())
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/provider-credentials",
    tag = "admin",
    request_body = serde_json::Value,
    responses(
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 501, description = "Admin CRUD not implemented (fail-closed)", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn upsert_provider_credential(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<(), ApiError> {
    state.require_http_admin(&headers)?;
    Err(provider_credentials_admin_not_implemented())
}

#[utoipa::path(
    put,
    path = "/api/v1/admin/provider-credentials/{provider_id}/{key_name}",
    tag = "admin",
    request_body = serde_json::Value,
    responses(
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 501, description = "Admin CRUD not implemented (fail-closed)", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn replace_provider_credential(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((_provider_id, _key_name)): Path<(String, String)>,
) -> Result<(), ApiError> {
    state.require_http_admin(&headers)?;
    Err(provider_credentials_admin_not_implemented())
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/provider-credentials/{provider_id}/{key_name}",
    tag = "admin",
    responses(
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 501, description = "Admin CRUD not implemented (fail-closed)", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn delete_provider_credential(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((_provider_id, _key_name)): Path<(String, String)>,
) -> Result<(), ApiError> {
    state.require_http_admin(&headers)?;
    Err(provider_credentials_admin_not_implemented())
}
