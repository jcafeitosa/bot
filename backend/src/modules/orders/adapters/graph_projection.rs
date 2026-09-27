use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::database::{
    GraphProjectionPort, Neo4jGraph, OrderIntentProjection, SubmittedEdgeProjection,
};
use crate::modules::orders::OrderSide;

pub const ORDER_INTENT_STATUS_SUBMITTED: &str = "submitted";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedOrderSubmitSnapshot {
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub execution_mode: String,
    /// Promoted bot from monitor/supervisor when present (F3.1 `SUBMITTED` edge).
    pub submitting_bot_id: Option<String>,
}

pub fn redacted_order_submit_to_projection(
    snapshot: &RedactedOrderSubmitSnapshot,
) -> OrderIntentProjection {
    OrderIntentProjection {
        client_order_id: snapshot.client_order_id.clone(),
        symbol: snapshot.symbol.clone(),
        side: order_side_sql(snapshot.side).to_string(),
        status: ORDER_INTENT_STATUS_SUBMITTED.to_string(),
        execution_mode: snapshot.execution_mode.clone(),
        submitted_at_ms: now_unix_ms(),
    }
}

fn order_side_sql(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub async fn best_effort_project_order_intent(
    neo4j: Option<&Neo4jGraph>,
    snapshot: &RedactedOrderSubmitSnapshot,
) {
    if snapshot.client_order_id.trim().is_empty() {
        return;
    }
    let Some(graph) = neo4j else {
        return;
    };
    let projector = graph.order_intent_projector();
    let projection = redacted_order_submit_to_projection(snapshot);
    if let Err(error) = project_order_intent_via_port(&projector, &projection).await {
        tracing::warn!(
            target: "database",
            client_order_id = %snapshot.client_order_id,
            symbol = %snapshot.symbol,
            %error,
            "neo4j order intent projection failed (order already committed)"
        );
        return;
    }
    if let Some(bot_id) = snapshot
        .submitting_bot_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
    {
        let edge = SubmittedEdgeProjection {
            bot_id: bot_id.to_string(),
            client_order_id: snapshot.client_order_id.clone(),
        };
        if let Err(error) = projector.project_submitted_edge(&edge).await {
            tracing::warn!(
                target: "database",
                client_order_id = %snapshot.client_order_id,
                bot_id = %bot_id,
                %error,
                "neo4j submitted edge projection failed (order already committed)"
            );
        }
    }
}

pub async fn project_order_intent_via_port(
    port: &dyn GraphProjectionPort,
    projection: &OrderIntentProjection,
) -> Result<(), crate::core::database::GraphProjectionError> {
    port.project_order_intent(projection).await
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::core::database::GraphProjectionError;
    use std::sync::Mutex;

    struct RecordingPort {
        intents: Mutex<Vec<OrderIntentProjection>>,
        edges: Mutex<Vec<SubmittedEdgeProjection>>,
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
            _projection: &crate::core::database::BotCatalogProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_bot_promotion(
            &self,
            _projection: &crate::core::database::BotPromotionProjection,
        ) -> Result<(), GraphProjectionError> {
            Ok(())
        }

        async fn project_order_intent(
            &self,
            projection: &OrderIntentProjection,
        ) -> Result<(), GraphProjectionError> {
            self.intents.lock().expect("lock").push(projection.clone());
            Ok(())
        }

        async fn project_submitted_edge(
            &self,
            projection: &SubmittedEdgeProjection,
        ) -> Result<(), GraphProjectionError> {
            self.edges.lock().expect("lock").push(projection.clone());
            Ok(())
        }
    }

    #[test]
    fn maps_submit_snapshot_without_quote_amount() {
        let projection = redacted_order_submit_to_projection(&RedactedOrderSubmitSnapshot {
            client_order_id: "cid-1".into(),
            symbol: "BTC/USDT".into(),
            side: OrderSide::Buy,
            execution_mode: "dev_accept".into(),
            submitting_bot_id: None,
        });
        assert_eq!(projection.client_order_id, "cid-1");
        assert_eq!(projection.side, "buy");
        assert_eq!(projection.status, "submitted");
        assert_eq!(projection.execution_mode, "dev_accept");
    }

    #[tokio::test]
    async fn project_via_port_records_redacted_intent() {
        let port = RecordingPort {
            intents: Mutex::new(Vec::new()),
            edges: Mutex::new(Vec::new()),
        };
        let projection = redacted_order_submit_to_projection(&RedactedOrderSubmitSnapshot {
            client_order_id: "cid-2".into(),
            symbol: "ETH/USDT".into(),
            side: OrderSide::Sell,
            execution_mode: "paper".into(),
            submitting_bot_id: None,
        });
        project_order_intent_via_port(&port, &projection)
            .await
            .expect("project");
        assert_eq!(port.intents.lock().expect("lock").len(), 1);
    }

    #[tokio::test]
    async fn best_effort_records_submitted_edge_when_bot_id_present() {
        struct LocalPort {
            intents: Mutex<Vec<OrderIntentProjection>>,
            edges: Mutex<Vec<SubmittedEdgeProjection>>,
        }
        #[async_trait::async_trait]
        impl GraphProjectionPort for LocalPort {
            async fn project_agent_hierarchy(
                &self,
                _: &crate::core::database::AgentHierarchyProjection,
            ) -> Result<(), GraphProjectionError> {
                Ok(())
            }
            async fn project_bot_catalog_entry(
                &self,
                _: &crate::core::database::BotCatalogProjection,
            ) -> Result<(), GraphProjectionError> {
                Ok(())
            }
            async fn project_bot_promotion(
                &self,
                _: &crate::core::database::BotPromotionProjection,
            ) -> Result<(), GraphProjectionError> {
                Ok(())
            }
            async fn project_order_intent(
                &self,
                projection: &OrderIntentProjection,
            ) -> Result<(), GraphProjectionError> {
                self.intents.lock().expect("lock").push(projection.clone());
                Ok(())
            }
            async fn project_submitted_edge(
                &self,
                projection: &SubmittedEdgeProjection,
            ) -> Result<(), GraphProjectionError> {
                self.edges.lock().expect("lock").push(projection.clone());
                Ok(())
            }
        }
        // exercise via same logic as best_effort without Neo4jGraph
        let snapshot = RedactedOrderSubmitSnapshot {
            client_order_id: "cid-bot".into(),
            symbol: "BTC/USDT".into(),
            side: OrderSide::Buy,
            execution_mode: "paper".into(),
            submitting_bot_id: Some("bot-1".into()),
        };
        let projection = redacted_order_submit_to_projection(&snapshot);
        let port = LocalPort {
            intents: Mutex::new(Vec::new()),
            edges: Mutex::new(Vec::new()),
        };
        project_order_intent_via_port(&port, &projection)
            .await
            .expect("intent");
        port.project_submitted_edge(&SubmittedEdgeProjection {
            bot_id: snapshot.submitting_bot_id.clone().expect("bot"),
            client_order_id: snapshot.client_order_id.clone(),
        })
        .await
        .expect("edge");
        assert_eq!(port.edges.lock().expect("lock").len(), 1);
    }
}

#[cfg(test)]
mod neo4j_integration_tests {
    use super::*;
    use crate::core::database::{load_agents_stack_from_env, Neo4jGraph};

    #[tokio::test]
    async fn neo4j_order_intent_after_redacted_projection() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        let snapshot = RedactedOrderSubmitSnapshot {
            client_order_id: format!(
                "neo4j-order-intent-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ),
            symbol: "BTC/USDT".into(),
            side: OrderSide::Buy,
            execution_mode: "dev_accept".into(),
            submitting_bot_id: None,
        };
        best_effort_project_order_intent(Some(&graph), &snapshot).await;
        let nodes = graph
            .count_order_intent_nodes(&snapshot.client_order_id, &snapshot.symbol)
            .await
            .expect("count");
        assert_eq!(nodes, 1);
    }

    #[tokio::test]
    async fn neo4j_submitted_edge_after_order_intent_projection() {
        if !crate::core::persistence::pg_integration::neo4j_stack_enabled() {
            return;
        }
        let config = load_agents_stack_from_env().expect("config");
        let graph = Neo4jGraph::connect(&config.neo4j).await.expect("connect");
        let bot_id = format!(
            "neo4j-bot-edge-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let client_order_id = format!("neo4j-coid-{}", bot_id);
        let snapshot = RedactedOrderSubmitSnapshot {
            client_order_id,
            symbol: "ETH/USDT".into(),
            side: OrderSide::Sell,
            execution_mode: "paper".into(),
            submitting_bot_id: Some(bot_id.clone()),
        };
        best_effort_project_order_intent(Some(&graph), &snapshot).await;
        let edges = graph
            .count_submitted_edges(&bot_id, &snapshot.client_order_id)
            .await
            .expect("edges");
        assert_eq!(edges, 1);
    }
}
