use crate::modules::orders::OrdersError;

/// Maps PostgreSQL/sqlx failures on order stores to fail-closed domain errors (HTTP 503).
pub(crate) fn orders_pg_store_error(operation: &str, error: sqlx::Error) -> OrdersError {
    OrdersError::StoreUnavailable(format!("{operation}: {error}"))
}
