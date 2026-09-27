//! Operational CLI for read-only Neo4j queries (F3).

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::json;

use super::config::{load_agents_stack_from_env, DatabaseConfigError};
use super::graph_query::{GraphQueryError, GraphQueryPort, Neo4jGraphQuery};
use super::neo4j::Neo4jGraph;

#[derive(Debug, Parser)]
pub struct GraphCli {
    #[command(subcommand)]
    pub command: GraphCommand,
}

#[derive(Debug, Subcommand)]
pub enum GraphCommand {
    /// Read-only graph queries (fail-closed without Neo4j agents stack).
    Query(GraphQueryCli),
}

#[derive(Debug, Parser)]
pub struct GraphQueryCli {
    #[command(subcommand)]
    pub command: GraphQueryCommand,
}

#[derive(Debug, Subcommand)]
pub enum GraphQueryCommand {
    /// List projected `Agent` nodes in the governance subgraph.
    Agents(GraphQueryAgentsCli),
    /// Supervision path from graph root to the target agent (F3 fatia 2).
    SupervisionChain(GraphQuerySupervisionChainCli),
    /// Bots promoted by the agent (`PROMOTED_BY` in the bots subgraph).
    BotsForAgent(GraphQueryBotsForAgentCli),
}

#[derive(Debug, Parser)]
pub struct GraphQueryAgentsCli {
    /// Maximum rows to return (1–500).
    #[arg(long, default_value_t = 32)]
    pub limit: u32,
}

#[derive(Debug, Parser)]
pub struct GraphQuerySupervisionChainCli {
    #[arg(long)]
    pub agency_id: String,
    #[arg(long)]
    pub agent_id: String,
}

#[derive(Debug, Parser)]
pub struct GraphQueryBotsForAgentCli {
    #[arg(long)]
    pub agency_id: String,
    #[arg(long)]
    pub agent_id: String,
    /// Maximum bot rows to return (1–500).
    #[arg(long, default_value_t = 32)]
    pub limit: u32,
}

pub async fn run(cli: &GraphCli) -> Result<()> {
    match &cli.command {
        GraphCommand::Query(args) => run_query(args).await,
    }
}

async fn run_query(cli: &GraphQueryCli) -> Result<()> {
    match &cli.command {
        GraphQueryCommand::Agents(args) => run_query_agents(args).await,
        GraphQueryCommand::SupervisionChain(args) => run_query_supervision_chain(args).await,
        GraphQueryCommand::BotsForAgent(args) => run_query_bots_for_agent(args).await,
    }
}

async fn run_query_agents(args: &GraphQueryAgentsCli) -> Result<()> {
    let neo4j = connect_neo4j_for_query().await?;
    let port = Neo4jGraphQuery::new(neo4j);
    let limit = args.limit.clamp(1, 500);
    let list = port.list_agents(limit).await.map_err(map_query_error)?;
    println!(
        "{}",
        json!({
            "agents": list.agents,
            "limit": limit,
        })
    );
    Ok(())
}

async fn run_query_supervision_chain(args: &GraphQuerySupervisionChainCli) -> Result<()> {
    let neo4j = connect_neo4j_for_query().await?;
    let port = Neo4jGraphQuery::new(neo4j);
    let chain = port
        .supervision_chain(&args.agency_id, &args.agent_id)
        .await
        .map_err(map_query_error)?;
    println!("{}", json!(chain));
    Ok(())
}

async fn run_query_bots_for_agent(args: &GraphQueryBotsForAgentCli) -> Result<()> {
    let neo4j = connect_neo4j_for_query().await?;
    let port = Neo4jGraphQuery::new(neo4j);
    let limit = args.limit.clamp(1, 500);
    let bots = port
        .bots_for_agent(&args.agency_id, &args.agent_id, limit)
        .await
        .map_err(map_query_error)?;
    println!(
        "{}",
        json!({
            "agency_id": bots.agency_id,
            "agent_id": bots.agent_id,
            "bots": bots.bots,
            "limit": limit,
        })
    );
    Ok(())
}

async fn connect_neo4j_for_query() -> Result<Neo4jGraph> {
    let stack = load_agents_stack_from_env()
        .map_err(|error| map_config_error("agents stack / Neo4j", error))?;
    if !stack.enabled {
        bail!(
            "graph query requires Neo4j agents stack (set BOT_AGENTS_ENABLED=true and BOT_NEO4J_* )"
        );
    }
    Neo4jGraph::connect(&stack.neo4j)
        .await
        .context("Neo4j connection failed for graph query")
}

fn map_config_error(label: &str, error: DatabaseConfigError) -> anyhow::Error {
    anyhow::anyhow!("invalid {label} configuration: {error}")
}

fn map_query_error(error: GraphQueryError) -> anyhow::Error {
    match error {
        GraphQueryError::Unavailable(message) => {
            anyhow::anyhow!("Neo4j unavailable (fail-closed): {message}")
        }
        GraphQueryError::Invalid(message) => anyhow::anyhow!("invalid graph query: {message}"),
        GraphQueryError::Driver(message) => anyhow::anyhow!("neo4j driver error: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn graph_cli_parses_query_agents_with_limit() {
        let cli =
            GraphCli::try_parse_from(["graph", "query", "agents", "--limit", "10"]).expect("parse");
        let GraphCommand::Query(GraphQueryCli {
            command: GraphQueryCommand::Agents(args),
        }) = cli.command
        else {
            panic!("expected agents query");
        };
        assert_eq!(args.limit, 10);
    }

    #[test]
    fn graph_cli_parses_query_supervision_chain() {
        let cli = GraphCli::try_parse_from([
            "graph",
            "query",
            "supervision-chain",
            "--agency-id",
            "agency-a",
            "--agent-id",
            "worker-1",
        ])
        .expect("parse");
        match cli.command {
            GraphCommand::Query(GraphQueryCli {
                command: GraphQueryCommand::SupervisionChain(args),
            }) => {
                assert_eq!(args.agency_id, "agency-a");
                assert_eq!(args.agent_id, "worker-1");
            }
            _ => panic!("expected supervision-chain"),
        }
    }

    #[test]
    fn graph_cli_parses_query_bots_for_agent() {
        let cli = GraphCli::try_parse_from([
            "graph",
            "query",
            "bots-for-agent",
            "--agency-id",
            "agency-a",
            "--agent-id",
            "agent-promoter",
            "--limit",
            "5",
        ])
        .expect("parse");
        match cli.command {
            GraphCommand::Query(GraphQueryCli {
                command: GraphQueryCommand::BotsForAgent(args),
            }) => {
                assert_eq!(args.agency_id, "agency-a");
                assert_eq!(args.agent_id, "agent-promoter");
                assert_eq!(args.limit, 5);
            }
            _ => panic!("expected bots-for-agent"),
        }
    }

    #[test]
    fn graph_cli_has_stable_help() {
        GraphCli::command().debug_assert();
    }

    #[test]
    fn graph_query_maps_driver_error() {
        let err = map_query_error(GraphQueryError::Driver("timeout".into()));
        assert!(err.to_string().contains("timeout"));
    }
}
