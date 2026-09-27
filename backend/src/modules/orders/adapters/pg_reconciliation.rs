use sqlx::PgPool;

use crate::core::database::PostgresDatabase;
use crate::modules::orders::models::{OrderSide, OrdersError, ReconciliationState};

/// Durable reconciliation rows when PostgreSQL is available (`order_reconciliation`).
#[derive(Clone)]
pub struct PgOrderReconciliationStore {
    pool: PgPool,
}

impl PgOrderReconciliationStore {
    pub fn new(db: &PostgresDatabase) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn mark_pending(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: OrderSide,
    ) -> Result<(), OrdersError> {
        let key = client_order_id.trim();
        if key.is_empty() || key.len() > 128 {
            return Err(OrdersError::InvalidRequest(
                "client_order_id must be 1..=128 bytes".into(),
            ));
        }
        let side_label = side_label(side);
        let result = sqlx::query(
            r#"
            INSERT INTO order_reconciliation (client_order_id, symbol, side, state)
            VALUES ($1, $2, $3, 'pending')
            ON CONFLICT (client_order_id) DO NOTHING
            "#,
        )
        .bind(key)
        .bind(symbol)
        .bind(side_label)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("reconciliation mark_pending failed: {error}"))
        })?;
        if result.rows_affected() == 0 {
            return Err(OrdersError::InvalidRequest(
                "client_order_id already tracked for reconciliation".into(),
            ));
        }
        Ok(())
    }

    pub async fn confirm_exchange_order(
        &self,
        client_order_id: &str,
        exchange_order_id: &str,
    ) -> Result<ReconciliationState, OrdersError> {
        let key = client_order_id.trim();
        let exchange_id = exchange_order_id.trim();
        if exchange_id.is_empty() {
            return Err(OrdersError::InvalidRequest(
                "exchange_order_id is required".into(),
            ));
        }
        let row = sqlx::query_as::<_, ReconciliationRow>(
            r#"
            UPDATE order_reconciliation
            SET state = 'reconciled',
                exchange_order_id = $2,
                updated_at = NOW()
            WHERE client_order_id = $1 AND state = 'pending'
            RETURNING client_order_id, state, exchange_order_id, divergent_reason
            "#,
        )
        .bind(key)
        .bind(exchange_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("reconciliation confirm failed: {error}"))
        })?;
        match row {
            Some(r) => Ok(r.into_state()),
            None => {
                let existing = self.state(key).await?;
                match existing {
                    Some(ReconciliationState::Reconciled { .. }) => Err(
                        OrdersError::InvalidRequest("client_order_id already reconciled".into()),
                    ),
                    Some(ReconciliationState::Divergent { .. }) => Err(
                        OrdersError::InvalidRequest("client_order_id marked divergent".into()),
                    ),
                    Some(ReconciliationState::Pending) => Err(OrdersError::InvalidRequest(
                        "reconciliation confirm race".into(),
                    )),
                    None => Err(OrdersError::InvalidRequest(
                        "unknown client_order_id for reconciliation".into(),
                    )),
                }
            }
        }
    }

    pub async fn state(
        &self,
        client_order_id: &str,
    ) -> Result<Option<ReconciliationState>, OrdersError> {
        let key = client_order_id.trim();
        let row = sqlx::query_as::<_, ReconciliationRow>(
            r#"
            SELECT client_order_id, state, exchange_order_id, divergent_reason
            FROM order_reconciliation
            WHERE client_order_id = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("reconciliation lookup failed: {error}"))
        })?;
        Ok(row.map(|r| r.into_state()))
    }

    /// Mirrors the in-process ledger row after submit (pending or reconciled in one upsert).
    pub async fn upsert_state(
        &self,
        client_order_id: &str,
        symbol: &str,
        side: OrderSide,
        state: &ReconciliationState,
    ) -> Result<(), OrdersError> {
        let key = client_order_id.trim();
        if key.is_empty() || key.len() > 128 {
            return Err(OrdersError::InvalidRequest(
                "client_order_id must be 1..=128 bytes".into(),
            ));
        }
        let side_label = side_label(side);
        let (state_label, exchange_id, reason) = match state {
            ReconciliationState::Pending => ("pending", None, None),
            ReconciliationState::Reconciled { exchange_order_id } => {
                ("reconciled", Some(exchange_order_id.as_str()), None)
            }
            ReconciliationState::Divergent { reason } => ("divergent", None, Some(reason.as_str())),
        };
        sqlx::query(
            r#"
            INSERT INTO order_reconciliation (client_order_id, symbol, side, state, exchange_order_id, divergent_reason)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (client_order_id) DO UPDATE SET
                symbol = EXCLUDED.symbol,
                side = EXCLUDED.side,
                state = EXCLUDED.state,
                exchange_order_id = EXCLUDED.exchange_order_id,
                divergent_reason = EXCLUDED.divergent_reason,
                updated_at = NOW()
            "#,
        )
        .bind(key)
        .bind(symbol)
        .bind(side_label)
        .bind(state_label)
        .bind(exchange_id)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("reconciliation upsert failed: {error}"))
        })?;
        Ok(())
    }

    /// All rows for in-process ledger hydration on HTTP `serve` boot.
    pub async fn list_for_memory_hydrate(
        &self,
    ) -> Result<Vec<(String, String, OrderSide, ReconciliationState)>, OrdersError> {
        let rows = sqlx::query_as::<_, ReconciliationHydrateRow>(
            r#"
            SELECT client_order_id, symbol, side, state, exchange_order_id, divergent_reason
            FROM order_reconciliation
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("reconciliation hydrate list failed: {error}"))
        })?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let ReconciliationHydrateRow {
                    client_order_id,
                    symbol,
                    side,
                    state,
                    exchange_order_id,
                    divergent_reason,
                } = row;
                (
                    client_order_id,
                    symbol,
                    parse_side_label(&side),
                    row_into_state(state, exchange_order_id, divergent_reason),
                )
            })
            .collect())
    }

    pub async fn mark_divergent(
        &self,
        client_order_id: &str,
        reason: &str,
    ) -> Result<(), OrdersError> {
        let key = client_order_id.trim();
        let result = sqlx::query(
            r#"
            UPDATE order_reconciliation
            SET state = 'divergent',
                divergent_reason = $2,
                updated_at = NOW()
            WHERE client_order_id = $1 AND state = 'pending'
            "#,
        )
        .bind(key)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("reconciliation mark_divergent failed: {error}"))
        })?;
        if result.rows_affected() == 0 {
            return Err(OrdersError::InvalidRequest(
                "reconciliation state cannot become divergent".into(),
            ));
        }
        Ok(())
    }

    pub async fn pending_count(&self) -> Result<usize, OrdersError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM order_reconciliation WHERE state = 'pending'",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            OrdersError::InvalidRequest(format!("reconciliation pending_count failed: {error}"))
        })?;
        Ok(count as usize)
    }
}

fn parse_side_label(label: &str) -> OrderSide {
    match label.trim().to_ascii_lowercase().as_str() {
        "sell" => OrderSide::Sell,
        _ => OrderSide::Buy,
    }
}

fn side_label(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    }
}

#[derive(sqlx::FromRow)]
struct ReconciliationRow {
    state: String,
    exchange_order_id: Option<String>,
    divergent_reason: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ReconciliationHydrateRow {
    client_order_id: String,
    symbol: String,
    side: String,
    state: String,
    exchange_order_id: Option<String>,
    divergent_reason: Option<String>,
}

fn row_into_state(
    state: String,
    exchange_order_id: Option<String>,
    divergent_reason: Option<String>,
) -> ReconciliationState {
    match state.as_str() {
        "pending" => ReconciliationState::Pending,
        "reconciled" => ReconciliationState::Reconciled {
            exchange_order_id: exchange_order_id.unwrap_or_else(|| "unknown".into()),
        },
        "divergent" => ReconciliationState::Divergent {
            reason: divergent_reason.unwrap_or_else(|| "unspecified".into()),
        },
        _ => ReconciliationState::Divergent {
            reason: format!("unknown state {}", state),
        },
    }
}

impl ReconciliationRow {
    fn into_state(self) -> ReconciliationState {
        row_into_state(self.state, self.exchange_order_id, self.divergent_reason)
    }
}

impl ReconciliationHydrateRow {
    fn into_state(self) -> ReconciliationState {
        row_into_state(self.state, self.exchange_order_id, self.divergent_reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at PostgreSQL 18+ database trading_bot with migrations applied"]
    async fn pg_order_reconciliation_round_trip() {
        use crate::core::persistence::Database;

        let db = Database::connect_from_env().await.expect("DATABASE_URL");
        db.migrate().await.expect("migrate");
        let store = PgOrderReconciliationStore::new(db.as_postgres());
        let key = format!(
            "recon-pg-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        store
            .mark_pending(&key, "BTC/USDT", OrderSide::Buy)
            .await
            .expect("pending");
        assert!(store.pending_count().await.expect("count") >= 1);
        let confirmed = store
            .confirm_exchange_order(&key, "ex-123")
            .await
            .expect("confirm");
        assert_eq!(
            confirmed,
            ReconciliationState::Reconciled {
                exchange_order_id: "ex-123".into(),
            }
        );
        let key2 = format!(
            "recon-pg-div-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        store
            .mark_pending(&key2, "ETH/USDT", OrderSide::Sell)
            .await
            .expect("pending2");
        store
            .mark_divergent(&key2, "exchange canceled")
            .await
            .expect("divergent");
        assert_eq!(
            store.state(&key2).await.expect("load div"),
            Some(ReconciliationState::Divergent {
                reason: "exchange canceled".into(),
            })
        );
    }
}
