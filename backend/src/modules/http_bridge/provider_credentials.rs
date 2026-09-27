use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::providers::credentials::{
    delete_provider_credential_row, list_masked, upsert, ProviderCredentialMasked,
    ProviderCredentialsStoreError,
};
use sqlx::PgPool;

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderCredentialMaskedBody {
    pub provider_id: String,
    pub key_name: String,
    pub secret_masked: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderCredentialsListResponse {
    pub credentials: Vec<ProviderCredentialMaskedBody>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertProviderCredentialRequest {
    pub provider_id: String,
    pub key_name: String,
    pub secret: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReplaceProviderCredentialRequest {
    pub secret: String,
}

fn to_body(row: ProviderCredentialMasked) -> ProviderCredentialMaskedBody {
    ProviderCredentialMaskedBody {
        provider_id: row.provider_id,
        key_name: row.key_name,
        secret_masked: row.secret_masked,
        updated_at: row.updated_at,
    }
}

pub async fn list_provider_credentials(
    pool: &PgPool,
) -> Result<ProviderCredentialsListResponse, ProviderCredentialsStoreError> {
    let credentials = list_masked(pool).await?.into_iter().map(to_body).collect();
    Ok(ProviderCredentialsListResponse { credentials })
}

pub async fn upsert_provider_credential(
    pool: &PgPool,
    body: UpsertProviderCredentialRequest,
) -> Result<ProviderCredentialMaskedBody, ProviderCredentialsStoreError> {
    let row = upsert(pool, &body.provider_id, &body.key_name, &body.secret).await?;
    Ok(to_body(row))
}

pub async fn replace_provider_credential(
    pool: &PgPool,
    provider_id: &str,
    key_name: &str,
    body: ReplaceProviderCredentialRequest,
) -> Result<ProviderCredentialMaskedBody, ProviderCredentialsStoreError> {
    let row = upsert(pool, provider_id, key_name, &body.secret).await?;
    Ok(to_body(row))
}

pub async fn delete_provider_credential(
    pool: &PgPool,
    provider_id: &str,
    key_name: &str,
) -> Result<(), ProviderCredentialsStoreError> {
    delete_provider_credential_row(pool, provider_id, key_name).await
}
