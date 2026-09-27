//! Provider API keys loaded from PostgreSQL (`provider_credentials`), with deprecated `.env` bootstrap fallback.

/// Observability seam: encryption-at-rest for `provider_credentials.secret` is not implemented.
/// Values are stored as plaintext in PostgreSQL until an ADR-backed encoder is wired.
pub const PROVIDER_CREDENTIALS_ENCRYPTION_MODE: &str = "none";

mod cache;
mod store;

pub use cache::{
    lookup_secret, reload_from_pool, KEY_API_KEY, PROVIDER_NGC, PROVIDER_NVIDIA, PROVIDER_OPENAI,
    PROVIDER_TYPESAFE,
};
pub use store::{
    delete as delete_provider_credential_row, list_masked, upsert, ProviderCredentialMasked,
    ProviderCredentialsStoreError,
};

#[cfg(test)]
mod pg_integration;

#[cfg(test)]
mod tests {
    use super::PROVIDER_CREDENTIALS_ENCRYPTION_MODE;

    #[test]
    fn provider_credentials_encryption_mode_is_explicit_none_fail_closed() {
        assert_eq!(PROVIDER_CREDENTIALS_ENCRYPTION_MODE, "none");
    }
}
