#[cfg(test)]
mod pg_integration {
    use super::super::cache::{lookup_secret, reload_from_pool, KEY_API_KEY, PROVIDER_TYPESAFE};

    #[tokio::test]
    #[ignore = "requires DATABASE_URL and migration 0007 applied"]
    async fn loads_credentials_from_postgres() {
        let db = crate::core::persistence::Database::connect_from_env()
            .await
            .expect("DATABASE_URL");
        db.migrate().await.expect("migrate");
        reload_from_pool(db.pool()).await.expect("reload");
        let _ = lookup_secret(PROVIDER_TYPESAFE, KEY_API_KEY, &[]);
    }
}
