use neo4rs::query;

use super::graph_projection::{
    GraphProjectionError, GraphProjectionPort, OrderIntentProjection, SubmittedEdgeProjection,
    BOTS_GRAPH_DOMAIN, TRADING_GRAPH_DOMAIN,
};
use super::neo4j::Neo4jGraph;

const MERGE_ORDER_INTENT: &str = r"
MERGE (o:OrderIntent {client_order_id: $client_order_id})
SET o.graph_domain = $graph_domain,
    o.symbol = $symbol,
    o.side = $side,
    o.status = $status,
    o.execution_mode = $execution_mode,
    o.submitted_at_ms = $submitted_at_ms
";

const MERGE_SUBMITTED_EDGE: &str = r"
MERGE (o:OrderIntent {client_order_id: $client_order_id})
SET o.graph_domain = $graph_domain
WITH o
MERGE (b:Bot {bot_id: $bot_id})
SET b.graph_domain = $bots_graph_domain
MERGE (b)-[:SUBMITTED]->(o)
";

#[derive(Clone)]
pub struct Neo4jOrderIntentProjector {
    graph: Neo4jGraph,
}

impl Neo4jOrderIntentProjector {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }
}

impl Neo4jGraph {
    pub fn order_intent_projector(&self) -> Neo4jOrderIntentProjector {
        Neo4jOrderIntentProjector::new(self.clone())
    }
}

#[async_trait::async_trait]
impl GraphProjectionPort for Neo4jOrderIntentProjector {
    async fn project_agent_hierarchy(
        &self,
        _projection: &super::graph_projection::AgentHierarchyProjection,
    ) -> Result<(), GraphProjectionError> {
        Ok(())
    }

    async fn project_bot_catalog_entry(
        &self,
        _projection: &super::graph_projection::BotCatalogProjection,
    ) -> Result<(), GraphProjectionError> {
        Ok(())
    }

    async fn project_bot_promotion(
        &self,
        _projection: &super::graph_projection::BotPromotionProjection,
    ) -> Result<(), GraphProjectionError> {
        Ok(())
    }

    async fn project_order_intent(
        &self,
        projection: &OrderIntentProjection,
    ) -> Result<(), GraphProjectionError> {
        validate_projection(projection)?;
        let q = query(MERGE_ORDER_INTENT)
            .param("client_order_id", projection.client_order_id.as_str())
            .param("graph_domain", TRADING_GRAPH_DOMAIN)
            .param("symbol", projection.symbol.as_str())
            .param("side", projection.side.as_str())
            .param("status", projection.status.as_str())
            .param("execution_mode", projection.execution_mode.as_str())
            .param("submitted_at_ms", projection.submitted_at_ms);
        self.graph
            .run_write(q)
            .await
            .map_err(|error| GraphProjectionError::Driver(error.to_string()))
    }

    async fn project_submitted_edge(
        &self,
        projection: &SubmittedEdgeProjection,
    ) -> Result<(), GraphProjectionError> {
        validate_submitted_edge(projection)?;
        let q = query(MERGE_SUBMITTED_EDGE)
            .param("client_order_id", projection.client_order_id.as_str())
            .param("graph_domain", TRADING_GRAPH_DOMAIN)
            .param("bot_id", projection.bot_id.as_str())
            .param("bots_graph_domain", BOTS_GRAPH_DOMAIN);
        self.graph
            .run_write(q)
            .await
            .map_err(|error| GraphProjectionError::Driver(error.to_string()))
    }
}

fn validate_projection(projection: &OrderIntentProjection) -> Result<(), GraphProjectionError> {
    if projection.client_order_id.trim().is_empty() {
        return Err(GraphProjectionError::Invalid(
            "client_order_id required".into(),
        ));
    }
    if projection.symbol.trim().is_empty() {
        return Err(GraphProjectionError::Invalid("symbol required".into()));
    }
    Ok(())
}

fn validate_submitted_edge(
    projection: &SubmittedEdgeProjection,
) -> Result<(), GraphProjectionError> {
    if projection.bot_id.trim().is_empty() {
        return Err(GraphProjectionError::Invalid("bot_id required".into()));
    }
    if projection.client_order_id.trim().is_empty() {
        return Err(GraphProjectionError::Invalid(
            "client_order_id required".into(),
        ));
    }
    Ok(())
}
