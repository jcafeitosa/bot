//! Operational CLI for graph projection outbox (F2.1.3).

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::json;
use tracing::info;

use crate::core::config::database::graph_projection_outbox_drain_batch;
use crate::core::persistence::Database;

use super::config::{load_agents_stack_from_env, postgres_url_from_env, DatabaseConfigError};
use super::graph_projection_outbox::{drain_graph_projection_outbox, GraphProjectionOutboxError};
use super::neo4j::Neo4jGraph;

#[derive(Debug, Parser)]
pub struct GraphProjectionCli {
    #[command(subcommand)]
    pub command: GraphProjectionCommand,
}

#[derive(Debug, Subcommand)]
pub enum GraphProjectionCommand {
    /// Drain pending/retry rows from `graph_projection_outbox` into Neo4j (fail-closed without PG+Neo4j).
    Drain(GraphProjectionDrainCli),
}

#[derive(Debug, Parser)]
pub struct GraphProjectionDrainCli {
    /// Maximum rows to process in this invocation (1–500).
    #[arg(long, default_value_t = default_drain_limit())]
    pub limit: u32,
}

fn default_drain_limit() -> u32 {
    graph_projection_outbox_drain_batch()
}

pub async fn run(cli: &GraphProjectionCli) -> Result<()> {
    match &cli.command {
        GraphProjectionCommand::Drain(args) => run_drain(args).await,
    }
}

pub async fn run_drain(args: &GraphProjectionDrainCli) -> Result<()> {
    let limit = args.limit.clamp(1, 500);
    let pool = connect_postgres_pool_for_drain().await?;
    let neo4j = connect_neo4j_for_drain().await?;

    let summary = drain_graph_projection_outbox(&pool, &neo4j, limit)
        .await
        .map_err(map_drain_error)?;

    info!(
        target: "graph_projection",
        processed = summary.processed,
        succeeded = summary.succeeded,
        failed = summary.failed,
        "graph projection outbox drain complete"
    );
    println!(
        "{}",
        json!({
            "processed": summary.processed,
            "succeeded": summary.succeeded,
            "failed": summary.failed,
        })
    );
    Ok(())
}

async fn connect_postgres_pool_for_drain() -> Result<sqlx::PgPool> {
    let url = postgres_url_from_env()
        .map_err(|error| map_config_error("DATABASE_URL", error))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "graph projection drain requires DATABASE_URL pointing at trading_bot (PostgreSQL not configured)"
            )
        })?;
    let db = Database::connect_from_url(&url)
        .await
        .context("failed to connect PostgreSQL for graph projection drain")?;
    db.migrate()
        .await
        .context("PostgreSQL migrations failed before graph projection drain")?;
    Ok(db.pool().clone())
}

async fn connect_neo4j_for_drain() -> Result<Neo4jGraph> {
    let stack = load_agents_stack_from_env()
        .map_err(|error| map_config_error("agents stack / Neo4j", error))?;
    if !stack.enabled {
        bail!(
            "graph projection drain requires Neo4j agents stack (set BOT_AGENTS_ENABLED=true and BOT_NEO4J_* )"
        );
    }
    Neo4jGraph::connect(&stack.neo4j)
        .await
        .context("Neo4j connection failed for graph projection drain")
}

fn map_config_error(label: &str, error: DatabaseConfigError) -> anyhow::Error {
    anyhow::anyhow!("invalid {label} configuration: {error}")
}

fn map_drain_error(error: GraphProjectionOutboxError) -> anyhow::Error {
    match error {
        GraphProjectionOutboxError::Neo4jUnavailable(message) => {
            anyhow::anyhow!("Neo4j unavailable (fail-closed, outbox unchanged): {message}")
        }
        GraphProjectionOutboxError::Store(message) => {
            anyhow::anyhow!("graph projection outbox store error: {message}")
        }
        GraphProjectionOutboxError::InvalidPayload(message) => {
            anyhow::anyhow!("invalid outbox payload: {message}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn graph_projection_cli_parses_drain_with_limit() {
        let cli =
            GraphProjectionCli::try_parse_from(["graph-projection", "drain", "--limit", "10"])
                .expect("parse");
        match cli.command {
            GraphProjectionCommand::Drain(args) => assert_eq!(args.limit, 10),
        }
    }

    #[test]
    fn graph_projection_cli_has_stable_help() {
        GraphProjectionCli::command().debug_assert();
    }

    #[test]
    fn graph_projection_drain_maps_neo4j_unavailable_fail_closed() {
        let err = map_drain_error(GraphProjectionOutboxError::Neo4jUnavailable(
            "ping failed".into(),
        ));
        let message = err.to_string();
        assert!(message.contains("fail-closed"));
        assert!(message.contains("Neo4j unavailable"));
    }
}
