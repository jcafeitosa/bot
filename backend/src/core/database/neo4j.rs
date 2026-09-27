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
    pub(crate) fn inner_graph(&self) -> &Graph {
        self.graph.as_ref()
    }

    pub(crate) async fn run_write(&self, q: neo4rs::Query) -> Result<(), Neo4jError> {
        self.inner_graph().run(q).await?;
        Ok(())
    }

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

    /// Test-only helper for F1 supervision chain assertions (modules integration tests).
    #[cfg(test)]
    pub async fn count_agent_supervision_paths(
        &self,
        agency_id: &str,
        from_agent_id: &str,
        to_agent_id: &str,
    ) -> Result<i64, Neo4jError> {
        let mut rows = self
            .inner_graph()
            .execute(
                query(
                    "MATCH (ceo:Agent {agent_id: $ceo_id, agency_id: $agency_id}) \
                     MATCH (worker:Agent {agent_id: $worker_id, agency_id: $agency_id}) \
                     MATCH (ceo)-[:SUPERVISES*]->(worker) RETURN count(worker) AS chains",
                )
                .param("ceo_id", from_agent_id)
                .param("worker_id", to_agent_id)
                .param("agency_id", agency_id),
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| Neo4jError::Probe("supervision count returned no row".into()))?;
        let chains: i64 = row
            .get("chains")
            .map_err(|error| Neo4jError::Probe(error.to_string()))?;
        Ok(chains)
    }

    /// Test-only helper for F2 `PROMOTED_BY` assertions (modules integration tests).
    #[cfg(test)]
    pub async fn count_bot_promoted_by_edges(
        &self,
        bot_id: &str,
        agent_id: &str,
        agency_id: &str,
    ) -> Result<i64, Neo4jError> {
        let mut rows = self
            .inner_graph()
            .execute(
                query(
                    "MATCH (a:Agent {agent_id: $agent_id, agency_id: $agency_id})                      -[:PROMOTED_BY]->(b:Bot {bot_id: $bot_id}) RETURN count(b) AS edges",
                )
                .param("agent_id", agent_id)
                .param("agency_id", agency_id)
                .param("bot_id", bot_id),
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| Neo4jError::Probe("promoted_by count returned no row".into()))?;
        let edges: i64 = row
            .get("edges")
            .map_err(|error| Neo4jError::Probe(error.to_string()))?;
        Ok(edges)
    }

    /// Test-only helper for F3 `OrderIntent` assertions (modules integration tests).
    #[cfg(test)]
    pub async fn count_order_intent_nodes(
        &self,
        client_order_id: &str,
        symbol: &str,
    ) -> Result<i64, Neo4jError> {
        let mut rows = self
            .inner_graph()
            .execute(
                query(
                    "MATCH (o:OrderIntent {client_order_id: $client_order_id, symbol: $symbol})                      RETURN count(o) AS nodes",
                )
                .param("client_order_id", client_order_id)
                .param("symbol", symbol),
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| Neo4jError::Probe("order intent count returned no row".into()))?;
        let nodes: i64 = row
            .get("nodes")
            .map_err(|error| Neo4jError::Probe(error.to_string()))?;
        Ok(nodes)
    }

    /// Test-only helper for F3.1 `SUBMITTED` edge assertions.
    #[cfg(test)]
    pub async fn count_submitted_edges(
        &self,
        bot_id: &str,
        client_order_id: &str,
    ) -> Result<i64, Neo4jError> {
        let mut rows = self
            .inner_graph()
            .execute(
                query(
                    "MATCH (b:Bot {bot_id: $bot_id})-[:SUBMITTED]->(o:OrderIntent {client_order_id: $client_order_id}) RETURN count(*) AS edges",
                )
                .param("bot_id", bot_id)
                .param("client_order_id", client_order_id),
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| Neo4jError::Probe("submitted edge count returned no row".into()))?;
        let edges: i64 = row
            .get("edges")
            .map_err(|error| Neo4jError::Probe(error.to_string()))?;
        Ok(edges)
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
    use crate::core::database::load_agents_stack_from_env;

    #[tokio::test]
    async fn ping_and_node_count_against_local_graph() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        graph.ping().await.expect("ping");
        assert!(graph.node_count().await.expect("count") > 0);
    }
}
