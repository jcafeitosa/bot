use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::database::{
    graph_projection_best_effort, BotCatalogProjection, BotPromotionProjection,
    GraphProjectionOutboxMessage, GraphProjectionPort, GraphProjectionSync, Neo4jGraph,
};
use crate::modules::bots::adapters::pg_catalog::operation_mode_to_sql;
use crate::modules::bots::models::{BotDefinition, BotPromotionRecord, BotPromotionState};

pub fn bot_definition_to_projection(definition: &BotDefinition) -> BotCatalogProjection {
    BotCatalogProjection {
        bot_id: definition.id.as_str().to_string(),
        strategy_id: definition.strategy_id.as_str().to_string(),
        strategy_version: definition.strategy_version.0,
        timeframe: definition.timeframe.clone(),
        symbol: definition.symbol.clone(),
        operation_mode: operation_mode_to_sql(definition.operation).to_string(),
        updated_at_ms: now_unix_ms(),
    }
}

pub fn promotion_record_to_projection(
    record: &BotPromotionRecord,
    agency_id: Option<&str>,
) -> BotPromotionProjection {
    BotPromotionProjection {
        bot_id: record.bot_id.clone(),
        promoted_by_agent_id: record.promoted_by.clone(),
        agency_id: agency_id.map(str::to_string),
        promotion_state: promotion_state_sql(record.state).to_string(),
        promoted_at_ms: record.promoted_at_unix_ms as i64,
    }
}

pub fn demotion_projection(bot_id: &str) -> BotPromotionProjection {
    BotPromotionProjection {
        bot_id: bot_id.to_string(),
        promoted_by_agent_id: "none".to_string(),
        agency_id: None,
        promotion_state: "demoted".to_string(),
        promoted_at_ms: now_unix_ms(),
    }
}

fn promotion_state_sql(state: BotPromotionState) -> &'static str {
    match state {
        BotPromotionState::Active => "active",
        BotPromotionState::Paused => "paused",
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn bot_catalog_graph_projection_messages(
    entries: &[BotDefinition],
) -> Vec<GraphProjectionOutboxMessage> {
    entries
        .iter()
        .map(|definition| {
            GraphProjectionOutboxMessage::bot_catalog(bot_definition_to_projection(definition))
        })
        .collect()
}

pub fn bot_promotion_graph_projection_message(
    record: &BotPromotionRecord,
    agency_id: Option<&str>,
) -> GraphProjectionOutboxMessage {
    GraphProjectionOutboxMessage::bot_promotion(promotion_record_to_projection(record, agency_id))
}

pub fn bot_demotion_graph_projection_message(bot_id: &str) -> GraphProjectionOutboxMessage {
    GraphProjectionOutboxMessage::bot_promotion(demotion_projection(bot_id))
}

pub async fn best_effort_project_bot_catalog(
    sync: GraphProjectionSync<'_>,
    entries: &[BotDefinition],
) {
    if sync.postgres.is_none() && sync.neo4j.is_none() {
        return;
    }
    let messages = bot_catalog_graph_projection_messages(entries);
    graph_projection_best_effort(sync, &messages).await;
}

pub async fn best_effort_project_bot_promotion(
    sync: GraphProjectionSync<'_>,
    record: &BotPromotionRecord,
    agency_id: Option<&str>,
) {
    if sync.postgres.is_none() && sync.neo4j.is_none() {
        return;
    }
    let message = bot_promotion_graph_projection_message(record, agency_id);
    graph_projection_best_effort(sync, &[message]).await;
}

pub async fn best_effort_retract_bot_promotion(sync: GraphProjectionSync<'_>, bot_id: &str) {
    if sync.postgres.is_none() && sync.neo4j.is_none() {
        return;
    }
    let message = bot_demotion_graph_projection_message(bot_id);
    graph_projection_best_effort(sync, &[message]).await;
}

#[allow(dead_code)]
pub async fn project_bot_catalog_via_port(
    port: &dyn GraphProjectionPort,
    definition: &BotDefinition,
) -> Result<(), crate::core::database::GraphProjectionError> {
    let projection = bot_definition_to_projection(definition);
    port.project_bot_catalog_entry(&projection).await
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::core::config::OperationMode;
    use crate::core::database::GraphProjectionError;
    use crate::modules::bots::models::{BotId, StrategyId, StrategyVersion};
    use std::sync::Mutex;

    struct RecordingPort {
        catalog: Mutex<Vec<BotCatalogProjection>>,
        promotions: Mutex<Vec<BotPromotionProjection>>,
    }

    #[async_trait::async_trait]
    impl GraphProjectionPort for RecordingPort {
        async fn project_agent_hierarchy(
            &self,
            _projection: &crate::core::database::AgentHierarchyProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_bot_catalog_entry(
            &self,
            projection: &BotCatalogProjection,
        ) -> Result<(), GraphProjectionError> {
            self.catalog.lock().expect("lock").push(projection.clone());
            Ok(())
        }

        async fn project_order_intent(
            &self,
            _projection: &crate::core::database::OrderIntentProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_submitted_edge(
            &self,
            _projection: &crate::core::database::SubmittedEdgeProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_bot_promotion(
            &self,
            projection: &BotPromotionProjection,
        ) -> Result<(), GraphProjectionError> {
            self.promotions
                .lock()
                .expect("lock")
                .push(projection.clone());
            Ok(())
        }
    }

    fn sample_definition() -> BotDefinition {
        let strategy_id = StrategyId::new("sma_cross").unwrap();
        let strategy_version = StrategyVersion(1);
        BotDefinition {
            id: BotId::new(&strategy_id, strategy_version, "1m", "BTC/USDT").unwrap(),
            strategy_id,
            strategy_version,
            timeframe: "1m".into(),
            symbol: "BTC/USDT".into(),
            operation: OperationMode::DayTrader,
        }
    }

    #[test]
    fn maps_definition_to_projection_without_secrets() {
        let projection = bot_definition_to_projection(&sample_definition());
        assert_eq!(projection.bot_id, "sma_cross@1:1m:BTC/USDT");
        assert_eq!(projection.strategy_id, "sma_cross");
        assert_eq!(projection.operation_mode, "day_trader");
    }

    #[tokio::test]
    async fn project_catalog_via_port_records_snapshot() {
        let port = RecordingPort {
            catalog: Mutex::new(Vec::new()),
            promotions: Mutex::new(Vec::new()),
        };
        project_bot_catalog_via_port(&port, &sample_definition())
            .await
            .expect("project");
        assert_eq!(port.catalog.lock().expect("lock").len(), 1);
    }
}

#[cfg(test)]
mod neo4j_integration_tests {
    use super::*;
    use crate::core::config::OperationMode;
    use crate::core::database::{load_agents_stack_from_env, Neo4jGraph};
    use crate::modules::bots::models::{
        BotId, BotPromotionRecord, BotPromotionState, StrategyId, StrategyVersion,
    };

    #[tokio::test]
    async fn neo4j_bot_promoted_by_after_catalog_and_promotion_projection() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        let strategy_id = StrategyId::new("ema_cross").unwrap();
        let strategy_version = StrategyVersion(1);
        let bot_id = BotId::new(&strategy_id, strategy_version, "1m", "BTC/USDT").expect("bot id");
        let definition = BotDefinition {
            id: bot_id.clone(),
            strategy_id,
            strategy_version,
            timeframe: "1m".into(),
            symbol: "BTC/USDT".into(),
            operation: OperationMode::DayTrader,
        };
        best_effort_project_bot_catalog(
            GraphProjectionSync {
                postgres: None,
                neo4j: Some(&graph),
            },
            &[definition],
        )
        .await;
        let record = BotPromotionRecord {
            bot_id: bot_id.as_str().to_string(),
            promoted_by: "agent-promoter".into(),
            promoted_at_unix_ms: 42,
            state: BotPromotionState::Active,
        };
        best_effort_project_bot_promotion(
            GraphProjectionSync {
                postgres: None,
                neo4j: Some(&graph),
            },
            &record,
            Some("agency-neo4j-test"),
        )
        .await;
        let edges = graph
            .count_bot_promoted_by_edges(bot_id.as_str(), "agent-promoter", "agency-neo4j-test")
            .await
            .expect("count");
        assert_eq!(edges, 1);
    }
}
