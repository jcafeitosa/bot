use std::sync::Arc;

use neo4rs::{query, Graph};

use super::config::Neo4jConnectionConfig;

pub struct Neo4jGraph {
    graph: Arc<Graph>,
}

impl std::fmt::Debug for Neo4jGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Neo4jGraph")
    }
}

impl Clone for Neo4jGraph {
    fn clone(&self) -> Self {
        Self {
            graph: Arc::clone(&self.graph),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Neo4jError {
    #[error("neo4j driver error: {0}")]
    Driver(#[from] neo4rs::Error),
    #[error("neo4j probe failed: {0}")]
    Probe(String),
}

impl Neo4jGraph {
    pub async fn connect(config: &Neo4jConnectionConfig) -> Result<Self, Neo4jError> {
        let graph = Graph::new(
            config.uri.as_str(),
            config.user.as_str(),
            config.password.as_str(),
        )
        .await?;
        Ok(Self {
            graph: Arc::new(graph),
        })
    }

    pub async fn ping(&self) -> Result<(), Neo4jError> {
        let mut rows = self.graph.execute(query("RETURN 1 AS ok")).await?;
        if rows.next().await?.is_some() {
            Ok(())
        } else {
            Err(Neo4jError::Probe("ping returned no row".into()))
        }
    }

    #[allow(dead_code)]
    pub async fn node_count(&self) -> Result<u64, Neo4jError> {
        let mut rows = self
            .graph
            .execute(query("MATCH (n) RETURN count(n) AS nodes"))
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| Neo4jError::Probe("count query returned no row".into()))?;
        let count: i64 = row
            .get("nodes")
            .map_err(|error| Neo4jError::Probe(error.to_string()))?;
        Ok(count.max(0) as u64)
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::core::database::config::load_agents_stack_from_env;

    #[tokio::test]
    #[ignore = "requires Neo4j docker-compose `graph` and BOT_AGENTS_ENABLED=true"]
    async fn ping_and_node_count_against_local_graph() {
        let config = load_agents_stack_from_env().expect("config");
        assert!(config.enabled);
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        graph.ping().await.expect("ping");
        assert!(graph.node_count().await.expect("count") > 0);
    }
}
