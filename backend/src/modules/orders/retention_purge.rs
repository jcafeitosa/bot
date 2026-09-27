//! Operational retention purge for Gate 2 orders tables (fail-closed; no live trading).

use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{PgPool, Row};

/// Policy aligned with `scripts/pg-orders-retention-purge.sql` and cli-and-config § PG orders retention.
#[derive(Debug, Clone, Copy)]
pub struct OrdersRetentionPolicy {
    pub idempotency_days: i32,
    pub reconciliation_terminal_days: i32,
    pub pending_stale_alert_days: i32,
}

impl Default for OrdersRetentionPolicy {
    fn default() -> Self {
        Self {
            idempotency_days: 90,
            reconciliation_terminal_days: 180,
            pending_stale_alert_days: 7,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PendingStaleReconciliation {
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrdersRetentionPurgeApplied {
    pub idempotency_rows_deleted: i64,
    pub reconciliation_terminal_rows_deleted: i64,
    pub pending_stale: Vec<PendingStaleReconciliation>,
    pub dry_run: bool,
}

pub async fn run_orders_retention_purge(
    pool: &PgPool,
    policy: OrdersRetentionPolicy,
    apply: bool,
) -> Result<OrdersRetentionPurgeApplied> {
    let pending_stale = list_pending_stale(pool, policy.pending_stale_alert_days).await?;
    if !apply {
        let idempotency_rows_deleted =
            count_idempotency_eligible(pool, policy.idempotency_days).await?;
        let reconciliation_terminal_rows_deleted =
            count_reconciliation_terminal_eligible(pool, policy.reconciliation_terminal_days)
                .await?;
        return Ok(OrdersRetentionPurgeApplied {
            idempotency_rows_deleted,
            reconciliation_terminal_rows_deleted,
            pending_stale,
            dry_run: true,
        });
    }

    let mut tx = pool
        .begin()
        .await
        .context("begin retention purge transaction")?;

    let idempotency_rows_deleted = sqlx::query_scalar(
        r#"
        WITH deleted AS (
            DELETE FROM order_idempotency_keys
            WHERE recorded_at < NOW() - make_interval(days => $1)
            RETURNING 1
        )
        SELECT COUNT(*)::bigint FROM deleted
        "#,
    )
    .bind(policy.idempotency_days)
    .fetch_one(&mut *tx)
    .await
    .context("delete expired order_idempotency_keys")?;

    let reconciliation_terminal_rows_deleted = sqlx::query_scalar(
        r#"
        WITH deleted AS (
            DELETE FROM order_reconciliation
            WHERE state IN ('reconciled', 'divergent')
              AND updated_at < NOW() - make_interval(days => $2)
            RETURNING 1
        )
        SELECT COUNT(*)::bigint FROM deleted
        "#,
    )
    .bind(policy.reconciliation_terminal_days)
    .fetch_one(&mut *tx)
    .await
    .context("delete expired terminal order_reconciliation rows")?;

    tx.commit()
        .await
        .context("commit retention purge transaction")?;

    Ok(OrdersRetentionPurgeApplied {
        idempotency_rows_deleted,
        reconciliation_terminal_rows_deleted,
        pending_stale,
        dry_run: false,
    })
}

async fn count_idempotency_eligible(pool: &PgPool, days: i32) -> Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM order_idempotency_keys
        WHERE recorded_at < NOW() - make_interval(days => $1)
        "#,
    )
    .bind(days)
    .fetch_one(pool)
    .await
    .context("count idempotency rows eligible for purge")
}

async fn count_reconciliation_terminal_eligible(pool: &PgPool, days: i32) -> Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM order_reconciliation
        WHERE state IN ('reconciled', 'divergent')
          AND updated_at < NOW() - make_interval(days => $1)
        "#,
    )
    .bind(days)
    .fetch_one(pool)
    .await
    .context("count terminal reconciliation rows eligible for purge")
}

async fn list_pending_stale(pool: &PgPool, days: i32) -> Result<Vec<PendingStaleReconciliation>> {
    let rows = sqlx::query(
        r#"
        SELECT client_order_id, symbol, side, updated_at::text AS updated_at
        FROM order_reconciliation
        WHERE state = 'pending'
          AND updated_at < NOW() - make_interval(days => $1)
        ORDER BY updated_at ASC
        LIMIT 100
        "#,
    )
    .bind(days)
    .fetch_all(pool)
    .await
    .context("list stale pending reconciliation rows")?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(PendingStaleReconciliation {
            client_order_id: row.try_get("client_order_id")?,
            symbol: row.try_get("symbol")?,
            side: row.try_get("side")?,
            updated_at: row.try_get("updated_at")?,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pg_orders_retention_purge_dry_run_then_apply_deletes_fixture_rows() {
        let Some(db) =
            crate::core::persistence::pg_integration::database_for_integration_test().await
        else {
            return;
        };
        let pool = db.pool().clone();
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let idem_key = format!("retention-purge-idem-{suffix}");
        let client_order_id = format!("retention-purge-rec-{suffix}");

        sqlx::query(
            r#"
            INSERT INTO order_idempotency_keys (client_order_id, recorded_at)
            VALUES ($1, NOW() - INTERVAL '91 days')
            "#,
        )
        .bind(&idem_key)
        .execute(&pool)
        .await
        .expect("insert stale idempotency row");

        sqlx::query(
            r#"
            INSERT INTO order_reconciliation (client_order_id, symbol, side, state, updated_at)
            VALUES ($1, 'BTCUSDT', 'buy', 'reconciled', NOW() - INTERVAL '181 days')
            "#,
        )
        .bind(&client_order_id)
        .execute(&pool)
        .await
        .expect("insert stale reconciliation row");

        let policy = OrdersRetentionPolicy::default();
        let dry = run_orders_retention_purge(&pool, policy, false)
            .await
            .expect("dry-run");
        assert!(dry.dry_run);
        assert!(dry.idempotency_rows_deleted >= 1);
        assert!(dry.reconciliation_terminal_rows_deleted >= 1);

        let applied = run_orders_retention_purge(&pool, policy, true)
            .await
            .expect("apply");
        assert!(!applied.dry_run);
        assert!(applied.idempotency_rows_deleted >= 1);
        assert!(applied.reconciliation_terminal_rows_deleted >= 1);

        let remaining_idem: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM order_idempotency_keys WHERE client_order_id = $1",
        )
        .bind(&idem_key)
        .fetch_one(&pool)
        .await
        .expect("count idem");
        assert_eq!(remaining_idem, 0);

        let remaining_rec: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM order_reconciliation WHERE client_order_id = $1",
        )
        .bind(&client_order_id)
        .fetch_one(&pool)
        .await
        .expect("count rec");
        assert_eq!(remaining_rec, 0);
    }
}
