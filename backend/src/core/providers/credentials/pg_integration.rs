#[cfg(test)]
mod tests {
    use super::super::cache::{lookup_secret, reload_from_pool, KEY_API_KEY, PROVIDER_TYPESAFE};
    use super::super::{list_masked, upsert};

    #[tokio::test]
    async fn loads_credentials_from_postgres() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let pool = db.pool();
        let secret = "pg-integration-typesafe-key-01";
        upsert(pool, PROVIDER_TYPESAFE, KEY_API_KEY, secret)
            .await
            .expect("upsert");
        reload_from_pool(pool).await.expect("reload");
        let value = lookup_secret(PROVIDER_TYPESAFE, KEY_API_KEY, &[]);
        assert_eq!(value.as_deref(), Some(secret));
        let listed = list_masked(pool).await.expect("list");
        let row = listed
            .iter()
            .find(|row| row.provider_id == PROVIDER_TYPESAFE)
            .expect("typesafe masked row");
        assert!(!row.secret_masked.contains(secret));
        assert!(row.secret_masked.contains("****"));
    }
}
