use neo4rs::query;

use super::graph_projection::{
    BotCatalogProjection, BotPromotionProjection, GraphProjectionError, GraphProjectionPort,
    AGENTS_GRAPH_DOMAIN, BOTS_GRAPH_DOMAIN,
};
use super::neo4j::Neo4jGraph;

const MERGE_BOT_CATALOG: &str = r"
MERGE (b:Bot {bot_id: $bot_id})
SET b.graph_domain = $graph_domain,
    b.strategy_id = $strategy_id,
    b.strategy_version = $strategy_version,
    b.timeframe = $timeframe,
    b.symbol = $symbol,
    b.operation_mode = $operation_mode,
    b.updated_at_ms = $updated_at_ms
WITH b
MERGE (s:Strategy {strategy_id: $strategy_id, version: $strategy_version})
SET s.graph_domain = $graph_domain
MERGE (b)-[:IMPLEMENTS]->(s)
";

const MERGE_BOT_PROMOTION: &str = r"
MERGE (b:Bot {bot_id: $bot_id})
SET b.graph_domain = $graph_domain,
    b.promotion_state = $promotion_state,
    b.promoted_at_ms = $promoted_at_ms
WITH b
OPTIONAL MATCH ()-[r:PROMOTED_BY]->(b)
DELETE r
WITH b
FOREACH (_ IN CASE WHEN $link_agent THEN [1] ELSE [] END |
  MERGE (a:Agent {agent_id: $agent_id, agency_id: $agency_id})
  SET a.graph_domain = $agents_graph_domain
  MERGE (a)-[:PROMOTED_BY]->(b)
)
";

#[derive(Clone)]
pub struct Neo4jBotProjector {
    graph: Neo4jGraph,
}

impl Neo4jBotProjector {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }
}

impl Neo4jGraph {
    pub fn bot_projector(&self) -> Neo4jBotProjector {
        Neo4jBotProjector::new(self.clone())
    }
}

#[async_trait::async_trait]
impl GraphProjectionPort for Neo4jBotProjector {
    async fn project_agent_hierarchy(
        &self,
        _projection: &super::graph_projection::AgentHierarchyProjection,
    ) -> Result<(), GraphProjectionError> {
        Ok(())
    }

    async fn project_bot_catalog_entry(
        &self,
        projection: &BotCatalogProjection,
    ) -> Result<(), GraphProjectionError> {
        validate_catalog_projection(projection)?;
        let q = query(MERGE_BOT_CATALOG)
            .param("bot_id", projection.bot_id.as_str())
            .param("graph_domain", BOTS_GRAPH_DOMAIN)
            .param("strategy_id", projection.strategy_id.as_str())
            .param("strategy_version", projection.strategy_version as i64)
            .param("timeframe", projection.timeframe.as_str())
            .param("symbol", projection.symbol.as_str())
            .param("operation_mode", projection.operation_mode.as_str())
            .param("updated_at_ms", projection.updated_at_ms);
        self.graph
            .run_write(q)
            .await
            .map_err(|error| GraphProjectionError::Driver(error.to_string()))
    }

    async fn project_bot_promotion(
        &self,
        projection: &BotPromotionProjection,
    ) -> Result<(), GraphProjectionError> {
        validate_promotion_projection(projection)?;
        let link_agent = projection
            .agency_id
            .as_deref()
            .is_some_and(|agency| !agency.is_empty());
        let q = query(MERGE_BOT_PROMOTION)
            .param("bot_id", projection.bot_id.as_str())
            .param("graph_domain", BOTS_GRAPH_DOMAIN)
            .param("promotion_state", projection.promotion_state.as_str())
            .param("promoted_at_ms", projection.promoted_at_ms)
            .param("link_agent", link_agent)
            .param("agent_id", projection.promoted_by_agent_id.as_str())
            .param("agency_id", projection.agency_id.as_deref().unwrap_or(""))
            .param("agents_graph_domain", AGENTS_GRAPH_DOMAIN);
        self.graph
            .run_write(q)
            .await
            .map_err(|error| GraphProjectionError::Driver(error.to_string()))
    }

    async fn project_order_intent(
        &self,
        _projection: &super::graph_projection::OrderIntentProjection,
    ) -> Result<(), GraphProjectionError> {
        Ok(())
    }

    async fn project_submitted_edge(
        &self,
        _projection: &super::graph_projection::SubmittedEdgeProjection,
    ) -> Result<(), GraphProjectionError> {
        Ok(())
    }
}

fn validate_catalog_projection(
    projection: &BotCatalogProjection,
) -> Result<(), GraphProjectionError> {
    if projection.bot_id.trim().is_empty() {
        return Err(GraphProjectionError::Invalid("bot_id required".into()));
    }
    if projection.strategy_id.trim().is_empty() {
        return Err(GraphProjectionError::Invalid("strategy_id required".into()));
    }
    Ok(())
}

fn validate_promotion_projection(
    projection: &BotPromotionProjection,
) -> Result<(), GraphProjectionError> {
    if projection.bot_id.trim().is_empty() {
        return Err(GraphProjectionError::Invalid("bot_id required".into()));
    }
    if projection.promoted_by_agent_id.trim().is_empty() {
        return Err(GraphProjectionError::Invalid(
            "promoted_by_agent_id required".into(),
        ));
    }
    Ok(())
}
