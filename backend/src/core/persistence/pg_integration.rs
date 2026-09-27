//! Optional PostgreSQL / Neo4j / testnet integration helpers for unit tests in the `bot` binary.
//! Tests call these and return early when prerequisites are missing (no `#[ignore]`).

use super::Database;

/// Connected, migrated `trading_bot` when `DATABASE_URL` is valid; otherwise `None` (skip test).
pub async fn database_for_integration_test() -> Option<Database> {
    match Database::connect_from_env().await {
        Ok(db) => match db.migrate().await {
            Ok(()) => Some(db),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

pub fn binance_testnet_credentials_configured() -> bool {
    crate::core::config::exchanges::binance_testnet_credentials_configured()
}

pub fn neo4j_stack_enabled() -> bool {
    crate::core::database::load_agents_stack_from_env()
        .map(|c| c.enabled)
        .unwrap_or(false)
}
