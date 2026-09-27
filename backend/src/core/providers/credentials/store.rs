//! PostgreSQL CRUD for `provider_credentials` (never log or expose `secret` on list).

use sqlx::PgPool;

use super::cache::reload_from_pool;
use super::{KEY_API_KEY, PROVIDER_NGC, PROVIDER_NVIDIA, PROVIDER_OPENAI, PROVIDER_TYPESAFE};

const MASKED_PREFIX: &str = "****";

/// Mask a secret for admin list responses (never return raw `secret`).
pub fn mask_secret(secret: &str) -> String {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return MASKED_PREFIX.to_string();
    }
    if trimmed.len() <= 4 {
        return MASKED_PREFIX.to_string();
    }
    let suffix = &trimmed[trimmed.len() - 4..];
    format!("{MASKED_PREFIX}{suffix}")
}

pub fn is_known_provider_id(provider_id: &str) -> bool {
    matches!(
        provider_id,
        PROVIDER_TYPESAFE | PROVIDER_OPENAI | PROVIDER_NVIDIA | PROVIDER_NGC
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCredentialMasked {
    pub provider_id: String,
    pub key_name: String,
    pub secret_masked: String,
    pub updated_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderCredentialsStoreError {
    #[error("invalid provider credential request: {0}")]
    InvalidRequest(String),
    #[error("provider credential not found")]
    NotFound,
    #[error("provider credentials store unavailable: {0}")]
    StoreUnavailable(String),
}

impl From<sqlx::Error> for ProviderCredentialsStoreError {
    fn from(err: sqlx::Error) -> Self {
        ProviderCredentialsStoreError::StoreUnavailable(err.to_string())
    }
}

fn normalize_id(value: &str, field: &str) -> Result<String, ProviderCredentialsStoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ProviderCredentialsStoreError::InvalidRequest(format!(
            "{field} must be non-empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn validate_upsert(
    provider_id: &str,
    key_name: &str,
    secret: &str,
) -> Result<(String, String, String), ProviderCredentialsStoreError> {
    let provider_id = normalize_id(provider_id, "provider_id")?;
    if !is_known_provider_id(&provider_id) {
        return Err(ProviderCredentialsStoreError::InvalidRequest(format!(
            "unknown provider_id '{provider_id}'"
        )));
    }
    let key_name = normalize_id(key_name, "key_name")?;
    if key_name != KEY_API_KEY {
        return Err(ProviderCredentialsStoreError::InvalidRequest(
            "only key_name 'api_key' is supported in v1".into(),
        ));
    }
    let secret = secret.trim();
    if secret.is_empty() {
        return Err(ProviderCredentialsStoreError::InvalidRequest(
            "secret must be non-empty".into(),
        ));
    }
    Ok((provider_id, key_name, secret.to_string()))
}

pub async fn list_masked(
    pool: &PgPool,
) -> Result<Vec<ProviderCredentialMasked>, ProviderCredentialsStoreError> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT provider_id, key_name, secret, updated_at::text \
         FROM provider_credentials \
         ORDER BY provider_id, key_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(provider_id, key_name, secret, updated_at)| ProviderCredentialMasked {
                provider_id,
                key_name,
                secret_masked: mask_secret(&secret),
                updated_at,
            },
        )
        .collect())
}

pub async fn upsert(
    pool: &PgPool,
    provider_id: &str,
    key_name: &str,
    secret: &str,
) -> Result<ProviderCredentialMasked, ProviderCredentialsStoreError> {
    let (provider_id, key_name, secret) = validate_upsert(provider_id, key_name, secret)?;
    let row = sqlx::query_as::<_, (String, String, String)>(
        "INSERT INTO provider_credentials (provider_id, key_name, secret) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (provider_id, key_name) \
         DO UPDATE SET secret = EXCLUDED.secret, updated_at = now() \
         RETURNING provider_id, key_name, updated_at::text",
    )
    .bind(&provider_id)
    .bind(&key_name)
    .bind(&secret)
    .fetch_one(pool)
    .await?;

    reload_from_pool(pool).await?;

    Ok(ProviderCredentialMasked {
        provider_id: row.0,
        key_name: row.1,
        secret_masked: mask_secret(&secret),
        updated_at: row.2,
    })
}

pub async fn delete(
    pool: &PgPool,
    provider_id: &str,
    key_name: &str,
) -> Result<(), ProviderCredentialsStoreError> {
    let provider_id = normalize_id(provider_id, "provider_id")?;
    let key_name = normalize_id(key_name, "key_name")?;
    let result =
        sqlx::query("DELETE FROM provider_credentials WHERE provider_id = $1 AND key_name = $2")
            .bind(&provider_id)
            .bind(&key_name)
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(ProviderCredentialsStoreError::NotFound);
    }

    reload_from_pool(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_secret_never_returns_full_value() {
        assert_eq!(mask_secret("abcdefghij"), "****ghij");
        assert_eq!(mask_secret("ab"), "****");
        assert_eq!(mask_secret("   "), "****");
    }

    #[test]
    fn validate_rejects_unknown_provider() {
        let err = validate_upsert("unknown", KEY_API_KEY, "secret").unwrap_err();
        assert!(matches!(
            err,
            ProviderCredentialsStoreError::InvalidRequest(_)
        ));
    }

    #[test]
    fn validate_accepts_typesafe_api_key() {
        let (pid, key, secret) =
            validate_upsert(PROVIDER_TYPESAFE, KEY_API_KEY, "  my-key  ").expect("ok");
        assert_eq!(pid, PROVIDER_TYPESAFE);
        assert_eq!(key, KEY_API_KEY);
        assert_eq!(secret, "my-key");
    }
}
