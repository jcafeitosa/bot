#![allow(dead_code)]
//! HTTP-facing facades so `presentation` does not import domain modules directly.

//! Facades consumed by the HTTP route layer (no direct domain imports in routes):
//! `agents`, `application`, `backtest`, `bots`, `bots_runtime`, `config`, `exchanges`, `monitor`, `orders`,
//! `portfolio`, `providers`, `risk`, `strategy`.

pub mod agents;
pub mod application;
pub mod backtest;
pub mod bots;
pub mod bots_runtime;
pub mod config;
pub mod exchanges;
pub mod monitor;
pub mod orders;
pub mod portfolio;
pub mod providers;
pub mod risk;
pub mod strategy;

#[cfg(test)]
mod bridge_tests {
    use crate::core::config::Config;

    #[tokio::test]
    async fn persist_catalog_bridge_wires_store_seam() {
        use crate::modules::bots::InMemoryBotCatalogStore;
        let config = Config::default();
        let mut store = InMemoryBotCatalogStore::new();
        let out = super::bots::persist_catalog_for_config(&config, &mut store)
            .await
            .expect("persist");
        assert!(out.persisted);
        assert!(!out.bots.is_empty());
    }

    #[tokio::test]
    async fn persist_catalog_bridge_materializes_monitor_registry_v2() {
        use crate::core::config::MonitorStrategyConfigEntry;
        use crate::modules::bots::InMemoryBotCatalogStore;
        let mut config = Config::default();
        config
            .strategy
            .monitor_registry
            .push(MonitorStrategyConfigEntry {
                id: "sma-cross".into(),
                version: 2,
                name: "SMA crossover v2".into(),
                fast_period: 3,
                slow_period: 15,
                evaluator: crate::modules::bots::MonitorEvaluatorKind::default(),
            });
        let mut store = InMemoryBotCatalogStore::new();
        let out = super::bots::persist_catalog_for_config(&config, &mut store)
            .await
            .expect("persist v2");
        assert!(out.bots.iter().any(|entry| entry.strategy_version == 2));
    }

    #[test]
    fn agent_snapshot_for_persist_requires_audit_event() {
        use crate::modules::agents::models::{
            AgencyId, AgentCapabilities, AgentId, AgentRole, OwnerId,
        };
        use crate::modules::agents::{AgentRegistry, NewAgentSpec, SupervisorRef};
        let mut registry = AgentRegistry::new();
        let spec = NewAgentSpec {
            id: AgentId::new("ceo").unwrap(),
            agency: AgencyId::new("acme").unwrap(),
            owner: OwnerId::new("owner-1").unwrap(),
            display_name: "CEO".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(OwnerId::new("owner-1").unwrap()),
            capabilities: AgentCapabilities::default(),
        };
        registry.register(spec, 1).expect("register");
        let snapshot = super::agents::snapshot_for_persist(&registry, "acme", "ceo").expect("snap");
        assert_eq!(snapshot.0.id.as_str(), "ceo");
    }

    #[test]
    fn agent_lifecycle_snapshot_for_persist_reflects_latest_audit_kind() {
        use crate::modules::agents::models::{
            AgencyId, AgentCapabilities, AgentId, AgentLifecycleState, AgentRole,
            IdentityEventKind, OwnerId,
        };
        use crate::modules::agents::{AgentRegistry, NewAgentSpec, SupervisorRef};

        let mut registry = AgentRegistry::new();
        let spec = NewAgentSpec {
            id: AgentId::new("ceo").unwrap(),
            agency: AgencyId::new("acme").unwrap(),
            owner: OwnerId::new("owner-1").unwrap(),
            display_name: "CEO".into(),
            role: AgentRole::Ceo,
            supervisor: SupervisorRef::Owner(OwnerId::new("owner-1").unwrap()),
            capabilities: AgentCapabilities::default(),
        };
        registry.register(spec, 1).expect("register");

        super::agents::pause(&mut registry, "acme", "ceo").expect("pause");
        let paused = super::agents::snapshot_for_persist(&registry, "acme", "ceo").expect("snap");
        assert_eq!(paused.0.state, AgentLifecycleState::Paused);
        assert_eq!(paused.1.kind, IdentityEventKind::Paused);

        super::agents::resume(&mut registry, "acme", "ceo").expect("resume");
        let resumed = super::agents::snapshot_for_persist(&registry, "acme", "ceo").expect("snap");
        assert_eq!(resumed.0.state, AgentLifecycleState::Active);
        assert_eq!(resumed.1.kind, IdentityEventKind::Resumed);

        super::agents::retire(&mut registry, "acme", "ceo").expect("retire");
        let retired = super::agents::snapshot_for_persist(&registry, "acme", "ceo").expect("snap");
        assert_eq!(retired.0.state, AgentLifecycleState::Retired);
        assert_eq!(retired.1.kind, IdentityEventKind::Retired);
    }

    #[tokio::test]
    async fn write_through_noops_without_postgres() {
        use crate::modules::agents::AgentRegistry;
        let registry = AgentRegistry::new();
        super::agents::write_through_agent_identity(&registry, None, "acme", "ceo")
            .await
            .expect("noop");
    }

    #[test]
    fn submit_order_http_fail_closed_returns_execution_disabled() {
        use crate::modules::http_bridge::orders::{OrderSideBody, SubmitOrderHttpRequest};
        use crate::modules::orders::{
            FailClosedExecutor, InMemoryOrderIdempotencyStore, OrdersError,
        };
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: crate::modules::http_bridge::risk::RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: None,
            paper_fill_unit_price: None,
        };
        let store = InMemoryOrderIdempotencyStore::new();
        let err =
            super::orders::submit_order_http(body, &FailClosedExecutor, &store, None).unwrap_err();
        assert!(matches!(err, OrdersError::ExecutionDisabled));
    }

    #[test]
    fn submit_order_http_succeeds_with_accepting_executor() {
        use crate::modules::http_bridge::orders::{OrderSideBody, SubmitOrderHttpRequest};
        use crate::modules::orders::{AcceptingExecutor, InMemoryOrderIdempotencyStore};
        let body = SubmitOrderHttpRequest {
            symbol: "BTC/USDT".into(),
            side: OrderSideBody::Buy,
            quote_amount: 5.0,
            estimated_daily_loss: 0.0,
            open_positions: 0,
            limits: crate::modules::http_bridge::risk::RiskLimitsBody {
                max_order_quote: 10.0,
                max_daily_loss_quote: 20.0,
                max_open_positions: 1,
            },
            client_order_id: None,
            paper_fill_unit_price: None,
        };
        let store = InMemoryOrderIdempotencyStore::new();
        let response = super::orders::submit_order_http(body, &AcceptingExecutor, &store, None)
            .expect("risk ok and executor accepts");
        assert!(response.accepted);
    }

    #[test]
    fn ranking_from_metrics_bridge_wires_full_ranking() {
        use crate::modules::bots::{
            BotId, BotMetrics, EvaluationWindow, RunId, StrategyId, StrategyVersion,
        };
        let strategy_id = StrategyId::new("sma-cross").expect("strategy");
        let bot_id =
            BotId::new(&strategy_id, StrategyVersion(1), "5m", "BTC/USDT").expect("bot_id");
        let metrics = vec![BotMetrics {
            bot_id,
            timeframe: "5m".into(),
            symbol: "BTC/USDT".into(),
            strategy_id,
            strategy_version: StrategyVersion(1),
            run_id: RunId("r1".into()),
            dataset_hash: "dataset-v1".into(),
            window: EvaluationWindow {
                start_ms: 100,
                end_ms: 200,
            },
            quote_currency: "USDT".into(),
            initial_capital_quote: 1000.0,
            net_pnl_quote: 20.0,
            net_return_pct: 2.0,
            max_drawdown_pct: 5.0,
            trades: 10,
            accuracy_pct: Some(92.0),
        }];
        let response = super::bots::ranking_from_metrics(metrics).expect("ranking");
        assert_eq!(response.report.entries.len(), 1);
        assert_eq!(response.report.entries[0].rank, 1);
    }

    #[test]
    fn bots_runtime_bridge_promote_and_demote_in_memory() {
        use crate::modules::bots::{BotRuntimePort, InMemoryBotRuntime, PromoteBotRequest};
        use std::sync::Arc;

        let runtime: Arc<dyn BotRuntimePort> = Arc::new(InMemoryBotRuntime::new());
        let status = super::bots_runtime::bot_runtime_status(runtime.as_ref());
        assert!(status.runtime_enabled);
        assert!(status.active.is_none());

        let record = super::bots_runtime::promote_bot(
            runtime.as_ref(),
            PromoteBotRequest {
                bot_id: "sma-cross@1:5m:BTC/USDT".into(),
                promoted_by: "integration-test".into(),
            },
        )
        .expect("promote");
        assert_eq!(
            super::bots_runtime::bot_runtime_status(runtime.as_ref()).active,
            Some(record)
        );
        super::bots_runtime::demote_bot(runtime.as_ref()).expect("demote");
        assert!(super::bots_runtime::bot_runtime_status(runtime.as_ref())
            .active
            .is_none());
    }
}
