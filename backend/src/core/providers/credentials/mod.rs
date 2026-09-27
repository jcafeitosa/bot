//! Provider API keys loaded from PostgreSQL (`provider_credentials`), with deprecated `.env` bootstrap fallback.

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
