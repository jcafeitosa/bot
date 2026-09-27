use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::modules::http_bridge::provider_credentials::{
    delete_provider_credential as bridge_delete_provider_credential,
    list_provider_credentials as bridge_list_provider_credentials,
    replace_provider_credential as bridge_replace_provider_credential,
    upsert_provider_credential as bridge_upsert_provider_credential, ProviderCredentialMaskedBody,
    ProviderCredentialsListResponse, ReplaceProviderCredentialRequest,
    UpsertProviderCredentialRequest,
};
use crate::presentation::http::{error::ApiError, state::ApiState};

fn require_postgres(state: &ApiState) -> Result<&sqlx::PgPool, ApiError> {
    let db = state.database().ok_or_else(|| {
        ApiError::with_code(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_credentials_store_unavailable",
            "PostgreSQL is not configured for provider credentials",
        )
    })?;
    Ok(db.pool())
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/provider-credentials",
    tag = "admin",
    responses(
        (status = 200, description = "Masked provider credentials", body = ProviderCredentialsListResponse),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL unavailable", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn list_provider_credentials(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<ProviderCredentialsListResponse>, ApiError> {
    state.require_http_admin(&headers)?;
    let pool = require_postgres(&state)?;
    bridge_list_provider_credentials(pool)
        .await
        .map_err(ApiError::from_provider_credentials_store_error)
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/provider-credentials",
    tag = "admin",
    request_body = UpsertProviderCredentialRequest,
    responses(
        (status = 200, description = "Credential upserted (masked)", body = ProviderCredentialMaskedBody),
        (status = 400, description = "Invalid request", body = crate::presentation::http::error::ApiErrorBody),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL unavailable", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn upsert_provider_credential(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<UpsertProviderCredentialRequest>,
) -> Result<Json<ProviderCredentialMaskedBody>, ApiError> {
    state.require_http_admin(&headers)?;
    let pool = require_postgres(&state)?;
    bridge_upsert_provider_credential(pool, body)
        .await
        .map_err(ApiError::from_provider_credentials_store_error)
        .map(Json)
}

#[utoipa::path(
    put,
    path = "/api/v1/admin/provider-credentials/{provider_id}/{key_name}",
    tag = "admin",
    request_body = ReplaceProviderCredentialRequest,
    responses(
        (status = 200, description = "Credential replaced (masked)", body = ProviderCredentialMaskedBody),
        (status = 400, description = "Invalid request", body = crate::presentation::http::error::ApiErrorBody),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL unavailable", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn replace_provider_credential(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((provider_id, key_name)): Path<(String, String)>,
    Json(body): Json<ReplaceProviderCredentialRequest>,
) -> Result<Json<ProviderCredentialMaskedBody>, ApiError> {
    state.require_http_admin(&headers)?;
    let pool = require_postgres(&state)?;
    bridge_replace_provider_credential(pool, &provider_id, &key_name, body)
        .await
        .map_err(ApiError::from_provider_credentials_store_error)
        .map(Json)
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/provider-credentials/{provider_id}/{key_name}",
    tag = "admin",
    responses(
        (status = 204, description = "Credential deleted"),
        (status = 401, description = "Missing admin bearer when BOT_HTTP_ADMIN_TOKEN is set", body = crate::presentation::http::error::ApiErrorBody),
        (status = 404, description = "Not found", body = crate::presentation::http::error::ApiErrorBody),
        (status = 503, description = "PostgreSQL unavailable", body = crate::presentation::http::error::ApiErrorBody)
    )
)]
pub async fn delete_provider_credential(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((provider_id, key_name)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    state.require_http_admin(&headers)?;
    let pool = require_postgres(&state)?;
    bridge_delete_provider_credential(pool, &provider_id, &key_name)
        .await
        .map_err(ApiError::from_provider_credentials_store_error)?;
    Ok(StatusCode::NO_CONTENT)
}
