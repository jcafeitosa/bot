//! Operational CLI for orders maintenance (retention purge; no live trading).

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::json;

use crate::core::database::{postgres_url_from_env, DatabaseConfigError};
use crate::core::persistence::Database;

use super::retention_purge::{run_orders_retention_purge, OrdersRetentionPolicy};

#[derive(Debug, Parser)]
pub struct OrdersCli {
    #[command(subcommand)]
    pub command: OrdersCommand,
}

#[derive(Debug, Subcommand)]
pub enum OrdersCommand {
    /// Report (default) or delete rows past retention on `order_idempotency_keys` / `order_reconciliation`.
    RetentionPurge(OrdersRetentionPurgeCli),
}

#[derive(Debug, Parser)]
pub struct OrdersRetentionPurgeCli {
    /// Delete eligible rows in a single transaction (default: dry-run counts only).
    #[arg(long)]
    pub apply: bool,
}

pub async fn run(cli: &OrdersCli) -> Result<()> {
    match &cli.command {
        OrdersCommand::RetentionPurge(args) => run_retention_purge(args).await,
    }
}

pub async fn run_retention_purge(args: &OrdersRetentionPurgeCli) -> Result<()> {
    let pool = connect_postgres_pool_for_orders_ops().await?;
    let summary = run_orders_retention_purge(&pool, OrdersRetentionPolicy::default(), args.apply)
        .await
        .context("orders retention purge failed")?;

    println!(
        "{}",
        json!({
            "dry_run": summary.dry_run,
            "idempotency_rows_deleted": summary.idempotency_rows_deleted,
            "reconciliation_terminal_rows_deleted": summary.reconciliation_terminal_rows_deleted,
            "pending_stale": summary.pending_stale,
        })
    );
    Ok(())
}

async fn connect_postgres_pool_for_orders_ops() -> Result<sqlx::PgPool> {
    let url = postgres_url_from_env()
        .map_err(|error| map_config_error("DATABASE_URL", error))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "orders retention purge requires DATABASE_URL pointing at trading_bot (PostgreSQL not configured)"
            )
        })?;
    if !url.contains("trading_bot") {
        anyhow::bail!(
            "orders retention purge requires DATABASE_URL to target database trading_bot"
        );
    }
    let db = Database::connect_from_url(&url)
        .await
        .context("failed to connect PostgreSQL for orders retention purge")?;
    db.migrate()
        .await
        .context("PostgreSQL migrations failed before orders retention purge")?;
    Ok(db.pool().clone())
}

fn map_config_error(label: &str, error: DatabaseConfigError) -> anyhow::Error {
    anyhow::anyhow!("invalid {label} configuration: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn orders_cli_parses_retention_purge_dry_run_by_default() {
        let cli = OrdersCli::try_parse_from(["orders", "retention-purge"]).expect("parse");
        match cli.command {
            OrdersCommand::RetentionPurge(args) => assert!(!args.apply),
        }
    }

    #[test]
    fn orders_cli_parses_retention_purge_apply() {
        let cli =
            OrdersCli::try_parse_from(["orders", "retention-purge", "--apply"]).expect("parse");
        match cli.command {
            OrdersCommand::RetentionPurge(args) => assert!(args.apply),
        }
    }

    #[test]
    fn orders_cli_has_stable_help() {
        OrdersCli::command().debug_assert();
    }
}
