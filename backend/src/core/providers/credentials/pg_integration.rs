#[cfg(test)]
mod tests {
    use super::super::cache::{lookup_secret, reload_from_pool, KEY_API_KEY, PROVIDER_TYPESAFE};

    #[tokio::test]
    async fn loads_credentials_from_postgres() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        reload_from_pool(db.pool()).await.expect("reload");
        let _ = lookup_secret(PROVIDER_TYPESAFE, KEY_API_KEY, &[]);
    }
}
