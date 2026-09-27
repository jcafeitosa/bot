use sqlx::PgPool;

use crate::core::config::ProductOwnerBootstrapConfig;
use crate::modules::agents::OwnerId;

const SINGLETON_ID: i16 = 1;

pub async fn load_bootstrapped_owner_id(pool: &PgPool) -> Result<Option<OwnerId>, String> {
    let row: Option<String> =
        sqlx::query_scalar("SELECT owner_id FROM product_owner_bootstrap WHERE singleton_id = $1")
            .bind(SINGLETON_ID)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    row.map(|raw| OwnerId::new(raw).map_err(|e| e.to_string()))
        .transpose()
}

pub async fn ensure_product_owner_bootstrapped(
    pool: &PgPool,
    config: &ProductOwnerBootstrapConfig,
    now_ms: i64,
) -> Result<Option<OwnerId>, String> {
    let existing = load_bootstrapped_owner_id(pool).await?;
    if config.explicit_bootstrap_requested() {
        let requested_raw = config
            .bootstrap_owner_id
            .as_deref()
            .expect("explicit_bootstrap_requested implies owner id");
        let requested = OwnerId::new(requested_raw).map_err(|e| e.to_string())?;
        if let Some(ref current) = existing {
            if current.as_str() != requested.as_str() {
                return Err(format!(
                    "product owner bootstrap conflict: database has {}, env requests {}",
                    current.as_str(),
                    requested.as_str()
                ));
            }
            return Ok(Some(requested));
        }
        insert_bootstrap(pool, &requested, now_ms).await?;
        return Ok(Some(requested));
    }
    Ok(existing)
}

async fn insert_bootstrap(pool: &PgPool, owner: &OwnerId, now_ms: i64) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO product_owner_bootstrap (singleton_id, owner_id, bootstrapped_at_ms, source) \
         VALUES ($1, $2, $3, 'env_explicit')",
    )
    .bind(SINGLETON_ID)
    .bind(owner.as_str())
    .bind(now_ms)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO product_owner_bootstrap_events (owner_id, kind, at_ms) VALUES ($1, 'bootstrapped', $2)",
    )
    .bind(owner.as_str())
    .bind(now_ms)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_sql_declares_product_owner_bootstrap_tables() {
        let sql =
            include_str!("../../../core/database/migrations/0010_product_owner_bootstrap.sql");
        assert!(sql.contains("product_owner_bootstrap"));
        assert!(sql.contains("product_owner_bootstrap_events"));
    }

    #[tokio::test]
    async fn pg_product_owner_bootstrap_idempotent_and_conflict_fail_closed() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let pool = db.pool();
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let owner_a = ProductOwnerBootstrapConfig {
            bootstrap_owner_id: Some(format!("owner-bootstrap-{suffix}")),
            bootstrap_ack: true,
        };
        let id_a = owner_a.bootstrap_owner_id.as_deref().unwrap();
        let first = ensure_product_owner_bootstrapped(pool, &owner_a, 1)
            .await
            .expect("bootstrap");
        assert_eq!(first.as_ref().map(|o| o.as_str()), Some(id_a));
        let second = ensure_product_owner_bootstrapped(pool, &owner_a, 2)
            .await
            .expect("idempotent");
        assert_eq!(second.as_ref().map(|o| o.as_str()), Some(id_a));

        let conflict = ProductOwnerBootstrapConfig {
            bootstrap_owner_id: Some("owner-other-conflict".into()),
            bootstrap_ack: true,
        };
        assert!(ensure_product_owner_bootstrapped(pool, &conflict, 3)
            .await
            .is_err());
    }
}
