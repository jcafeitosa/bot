use std::net::SocketAddr;
use std::time::Duration;

use axum::{routing::get, Json, Router};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::modules::agents::shared_agent_registry;
use crate::modules::config_api::Config;
use crate::presentation::http::openapi::ApiDoc;
use crate::presentation::http::routes;
use crate::presentation::http::state::ApiState;

pub async fn run(
    bind: SocketAddr,
    app_config: Config,
    monitor: Option<crate::modules::monitor::MonitorHandle>,
) -> anyhow::Result<()> {
    let databases = crate::core::database::AppDatabases::bootstrap_runtime().await;
    databases.spawn_graph_projection_outbox_worker();
    let agents = shared_agent_registry();
    let state =
        ApiState::build_api_state_for_http_serve(monitor, databases, app_config, agents).await;
    let monitor_attached = state.monitor().is_some();
    let http_admin_auth_enabled = state.http_admin_auth_enabled();
    let order_execution_mode = state.order_execution_mode();
    let bot_runtime_enabled = state.bot_runtime_status().runtime_enabled;
    if let Some(interval_secs) = order_reconciliation_poll_interval_secs() {
        if state.live_exchange_wired() {
            let poll_state = state.clone();
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    ticker.tick().await;
                    if let Err(error) = poll_state.run_order_reconciliation_poll_once().await {
                        tracing::warn!(
                            target: "api",
                            %error,
                            "order reconciliation poll tick failed"
                        );
                    }
                }
            });
            tracing::info!(
                target: "api",
                interval_secs,
                "order reconciliation background poll enabled"
            );
        }
    }
    if state.database().is_some() {
        tracing::info!(
            target: "api",
            "HTTP boot completed PostgreSQL hydrate (agents, reconciliation, bot catalog)"
        );
    }
    let app = build_router(state);

    let listener = TcpListener::bind(bind).await?;
    tracing::info!(
        target: "api",
        %bind,
        monitor_attached,
        http_admin_auth_enabled,
        ?order_execution_mode,
        bot_runtime_enabled,
        openapi_paths = ApiDoc::openapi().paths.paths.len(),
        "HTTP API listening (Scalar at /docs)"
    );
    axum::serve(listener, app).await?;
    Ok(())
}

fn order_reconciliation_poll_interval_secs() -> Option<u64> {
    crate::core::config::order_reconciliation_poll_interval_secs()
}

pub fn build_router(state: ApiState) -> Router {
    let openapi = ApiDoc::openapi();
    let openapi_route = openapi.clone();
    Router::new()
        .merge(routes::system_routes())
        .nest("/api/v1", routes::v1_routes())
        .route(
            "/openapi.json",
            get(move || async move { Json(openapi_route) }),
        )
        .merge(Scalar::with_url("/docs", openapi))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_and_openapi_paths_are_registered() {
        let app = build_router(ApiState::default());
        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);

        let openapi = app
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(openapi.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(openapi.into_body(), usize::MAX)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["info"]["title"], "Rust Trading Bot API");
        assert!(doc["paths"]["/api/v1/backtest/sma-crossover"].is_object());
    }

    #[tokio::test]
    async fn agents_register_and_openapi_lists_new_paths() {
        let app = build_router(ApiState::default());
        let body = r#"{"agency":"acme","owner_id":"owner-1","agent_id":"ceo","display_name":"CEO","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"}}"#;
        let register = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register.status(), StatusCode::CREATED);

        let openapi = app
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(openapi.into_body(), usize::MAX)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let paths = doc["paths"].as_object().expect("paths").len();
        assert!(
            paths >= 29,
            "expected expanded openapi surface, got {paths}"
        );
        assert!(doc["paths"]["/api/v1/agents"].is_object());
        assert!(doc["paths"]["/api/v1/risk/gate-signal"].is_object());
    }

    #[tokio::test]
    async fn documented_get_routes_respond() {
        let app = build_router(ApiState::default());
        let get_paths = [
            ("/healthz", StatusCode::OK),
            ("/readyz", StatusCode::OK),
            ("/api/v1/meta", StatusCode::OK),
            ("/api/v1/application/signals", StatusCode::OK),
            ("/api/v1/config/active", StatusCode::OK),
            (
                "/api/v1/config/snapshot?config=src/core/config/bot.toml",
                StatusCode::OK,
            ),
            ("/api/v1/providers/status", StatusCode::OK),
            ("/api/v1/exchanges/catalog", StatusCode::OK),
            ("/api/v1/exchanges/routing", StatusCode::OK),
            (
                "/api/v1/strategy/periods?operation=day_trader",
                StatusCode::OK,
            ),
            (
                "/api/v1/portfolio/paper-snapshot?quote=usdt",
                StatusCode::OK,
            ),
            ("/api/v1/monitor/snapshot", StatusCode::SERVICE_UNAVAILABLE),
            ("/api/v1/agents?agency=acme", StatusCode::OK),
            ("/api/v1/agents/audit?agency=acme", StatusCode::OK),
            ("/api/v1/bots/catalog", StatusCode::OK),
            ("/api/v1/bots/runtime/status", StatusCode::OK),
            ("/api/v1/orders/execution-status", StatusCode::OK),
            (
                "/api/v1/orders/reconciliation/doc-smoke-missing",
                StatusCode::NOT_FOUND,
            ),
        ];
        for (path, expected) in get_paths {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "unexpected status for {path}");
        }

        let monitor_cmd = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/monitor/commands")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"command":"pause"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(monitor_cmd.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn documented_post_routes_accept_valid_json() {
        let app = build_router(ApiState::default());
        let posts = [
            (
                "/api/v1/risk/profile-limits",
                r#"{"profile":"conservative","base":{"max_order_quote":100.0,"max_daily_loss_quote":50.0,"max_open_positions":3}}"#,
            ),
            (
                "/api/v1/risk/validate-intent",
                r#"{"intent":{"quote_amount":5.0,"estimated_daily_loss":0.0,"open_positions":0},"limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1}}"#,
            ),
            (
                "/api/v1/risk/gate-signal",
                r#"{"signal":"hold","limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1},"run_mode":"paper"}"#,
            ),
            (
                "/api/v1/strategy/evaluate-sma",
                r#"{"fast_period":2,"slow_period":3,"candles":[{"timestamp_ms":1,"open":1.0,"high":1.0,"low":1.0,"close":1.0,"volume":1.0},{"timestamp_ms":2,"open":1.0,"high":1.0,"low":1.0,"close":2.0,"volume":1.0},{"timestamp_ms":3,"open":2.0,"high":2.0,"low":2.0,"close":3.0,"volume":1.0}]}"#,
            ),
            (
                "/api/v1/backtest/sma-crossover",
                r#"{"config":"src/core/config/bot.toml","persist":false}"#,
            ),
        ];
        for (path, body) in posts {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "unexpected status for {path}"
            );
        }
    }

    #[tokio::test]
    async fn router_after_build_api_state_serves_catalog_and_meta() {
        use crate::core::database::AppDatabases;

        let state = ApiState::build_api_state_for_http_serve(
            None,
            AppDatabases::empty(),
            crate::modules::config_api::Config::default(),
            std::sync::Arc::new(std::sync::Mutex::new(
                crate::modules::agents::AgentRegistry::new(),
            )),
        )
        .await;
        let app = build_router(state);
        for path in ["/api/v1/meta", "/api/v1/bots/catalog"] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "unexpected status for {path}"
            );
        }
    }

    #[tokio::test]
    async fn router_after_build_api_state_paper_submit_updates_portfolio() {
        use crate::core::database::AppDatabases;
        use crate::core::test_env_lock::EnvTestGuard;
        use crate::modules::orders::{PaperLedgerExecutor, PaperLedgerTestGuard};

        let _env = EnvTestGuard::acquire();
        let _paper_ledger = PaperLedgerTestGuard::acquire();
        std::env::set_var("BOT_ORDERS_EXECUTION", "paper");
        PaperLedgerExecutor::clear_ledger();

        let state = ApiState::build_api_state_for_http_serve(
            None,
            AppDatabases::empty(),
            crate::modules::config_api::Config::default(),
            std::sync::Arc::new(std::sync::Mutex::new(
                crate::modules::agents::AgentRegistry::new(),
            )),
        )
        .await;
        let app = build_router(state);
        let submit_body = r#"{"symbol":"BTC/USDT","side":"buy","quote_amount":100.0,"estimated_daily_loss":0.0,"open_positions":0,"paper_fill_unit_price":50000.0,"limits":{"max_order_quote":200.0,"max_daily_loss_quote":20.0,"max_open_positions":1}}"#;
        let submit = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/orders/submit")
                    .header("content-type", "application/json")
                    .body(Body::from(submit_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(submit.status(), StatusCode::OK);
        let snapshot = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/portfolio/paper-snapshot?quote=usdt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(snapshot.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(snapshot.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["available"], "900");
        assert_eq!(json["positions"].as_array().map(|a| a.len()), Some(1));
        PaperLedgerExecutor::clear_ledger();
        std::env::remove_var("BOT_ORDERS_EXECUTION");
    }

    #[tokio::test]
    async fn router_after_build_api_state_meta_agrees_with_http_seam_endpoints() {
        use crate::core::database::AppDatabases;

        let state = ApiState::build_api_state_for_http_serve(
            None,
            AppDatabases::empty(),
            crate::modules::config_api::Config::default(),
            std::sync::Arc::new(std::sync::Mutex::new(
                crate::modules::agents::AgentRegistry::new(),
            )),
        )
        .await;
        let app = build_router(state);
        let meta_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/meta")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(meta_response.status(), StatusCode::OK);
        let meta_bytes = axum::body::to_bytes(meta_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&meta_bytes).unwrap();
        let runtime_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/bots/runtime/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(runtime_response.status(), StatusCode::OK);
        let runtime_bytes = axum::body::to_bytes(runtime_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let runtime: serde_json::Value = serde_json::from_slice(&runtime_bytes).unwrap();
        assert_eq!(
            meta["http_seams"]["bot_runtime_enabled"],
            runtime["runtime_enabled"]
        );
        let exec_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/orders/execution-status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(exec_response.status(), StatusCode::OK);
        let exec_bytes = axum::body::to_bytes(exec_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let exec: serde_json::Value = serde_json::from_slice(&exec_bytes).unwrap();
        assert_eq!(meta["http_seams"]["order_execution_mode"], exec["mode"]);
        assert_eq!(
            meta["http_seams"]["live_exchange_wired"],
            exec["live_exchange_wired"]
        );
    }

    #[test]
    fn order_reconciliation_poll_interval_parses_positive_seconds() {
        use crate::core::test_env_lock::with_env_test_lock;
        with_env_test_lock(|| {
            std::env::set_var("BOT_ORDERS_RECONCILIATION_POLL_SECS", "30");
            assert_eq!(order_reconciliation_poll_interval_secs(), Some(30));
            std::env::remove_var("BOT_ORDERS_RECONCILIATION_POLL_SECS");
        });
    }

    #[tokio::test]
    async fn bots_ranking_post_returns_report_shape() {
        let app = build_router(ApiState::default());
        let body = r#"{"metrics":[{"bot_id":"sma-cross@1:5m:BTC/USDT","timeframe":"5m","symbol":"BTC/USDT","strategy_id":"sma-cross","strategy_version":1,"run_id":"r1","dataset_hash":"dataset-v1","window":{"start_ms":100,"end_ms":200},"quote_currency":"USDT","initial_capital_quote":1000.0,"net_pnl_quote":20.0,"net_return_pct":2.0,"max_drawdown_pct":5.0,"trades":10,"accuracy_pct":92.0}]}"#;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bots/ranking")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let report = doc.get("report").expect("report");
        assert_eq!(report["dataset_hash"], "dataset-v1");
        assert_eq!(report["quote_currency"], "USDT");
        let entries = report["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["rank"], 1);
    }

    #[tokio::test]
    async fn bots_catalog_persist_returns_persisted_flag() {
        let app = build_router(ApiState::default());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bots/catalog/persist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["persisted"], true);
        assert!(doc["bots"].as_array().is_some_and(|a| !a.is_empty()));
    }

    #[tokio::test]
    async fn orders_submit_risk_rejected_returns_422() {
        let app = build_router(ApiState::default());
        let body = r#"{"symbol":"BTC/USDT","side":"buy","quote_amount":500.0,"estimated_daily_loss":0.0,"open_positions":0,"limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1}}"#;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/orders/submit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn orders_submit_fail_closed_returns_503() {
        let app = build_router(ApiState::default());
        let body = r#"{"symbol":"BTC/USDT","side":"buy","quote_amount":5.0,"estimated_daily_loss":0.0,"open_positions":0,"limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1}}"#;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/orders/submit")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn openapi_surface_lists_core_paths() {
        let doc = ApiDoc::openapi();
        let paths = &doc.paths.paths;
        assert!(
            paths.len() >= 40,
            "expected at least 40 openapi paths, got {}",
            paths.len()
        );
        for key in [
            "/api/v1/config/active",
            "/api/v1/agents/{agent_id}/advisory",
            "/api/v1/bots/catalog",
            "/api/v1/bots/ranking",
            "/api/v1/monitor/commands",
            "/api/v1/risk/gate-signal",
            "/api/v1/orders/submit",
            "/api/v1/orders/reconciliation/{client_order_id}",
            "/api/v1/admin/provider-credentials",
            "/api/v1/admin/graph/agents",
            "/api/v1/admin/graph/supervision-chain",
            "/api/v1/admin/graph/bots-for-agent",
            "/api/v1/admin/graph/code-impact",
        ] {
            assert!(paths.contains_key(key), "missing openapi path {key}");
        }
    }

    #[tokio::test]
    async fn monitor_routes_use_attached_handle() {
        use crate::modules::config_api::Config;
        use crate::modules::monitor::{MonitorCommand, MonitorHandle, MonitorSnapshot};

        let (handle, mut commands, _) = MonitorHandle::channel(8, 64);
        let _snapshot_subscriber = handle.latest_snapshot();
        let _event_subscriber = handle.subscribe();
        let mut snapshot = MonitorSnapshot::initial();
        snapshot.revision = 7;
        snapshot.symbol = "BTC/USDT".to_string();
        handle.publish_snapshot(snapshot).unwrap();

        let app = build_router(ApiState::new(
            Some(handle),
            crate::core::database::AppDatabases::empty(),
            None,
            Config::default(),
        ));

        let snap_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/monitor/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(snap_resp.status(), StatusCode::OK);

        let cmd_resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/monitor/commands")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"command":"pause"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cmd_resp.status(), StatusCode::ACCEPTED);
        assert!(matches!(commands.recv().await, Some(MonitorCommand::Pause)));
    }

    #[tokio::test]
    async fn agents_lifecycle_endpoints_after_register() {
        let app = build_router(ApiState::default());
        let register_body = r#"{"agency":"lifecycle","owner_id":"owner-1","agent_id":"ceo","display_name":"CEO","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"},"consult_jev":false}"#;
        let register = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(register_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register.status(), StatusCode::CREATED);

        let get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/agents/ceo?agency=lifecycle")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::OK);

        let pause = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/agents/ceo/pause?agency=lifecycle")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pause.status(), StatusCode::OK);
    }
}
