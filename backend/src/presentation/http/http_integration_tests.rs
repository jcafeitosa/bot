//! HTTP integration tests referenced by docs/reference/test-matrix.md (admin bearer, orders executors).

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use tower::ServiceExt;

use crate::core::database::AppDatabases;
use crate::core::test_env_lock::EnvTestGuard;
use crate::modules::agents::AgentRegistry;
use crate::modules::config_api::Config;
use crate::modules::orders::{PaperLedgerExecutor, PaperLedgerTestGuard};
use crate::presentation::http::admin_auth::HttpAdminAuth;
use crate::presentation::http::order_execution::HttpOrderExecutor;
use crate::presentation::http::server::build_router;
use crate::presentation::http::state::ApiState;

const ADMIN_TOKEN: &str = "integration-admin-token";

const AGENT_REGISTER_JSON: &str = r#"{"agency":"acme","owner_id":"owner-1","agent_id":"ceo","display_name":"CEO","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"},"consult_jev":false}"#;

const ORDER_SUBMIT_JSON: &str = r#"{"symbol":"BTC/USDT","side":"buy","quote_amount":5.0,"estimated_daily_loss":0.0,"open_positions":0,"limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1}}"#;

fn fresh_agents() -> Arc<Mutex<AgentRegistry>> {
    Arc::new(Mutex::new(AgentRegistry::new()))
}

fn router_with_admin(auth: HttpAdminAuth) -> Router {
    build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        auth,
    ))
}

fn bearer_header(token: &str) -> (&'static str, String) {
    ("authorization", format!("Bearer {}", token))
}

#[tokio::test]
async fn agents_register_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(bearer_header(ADMIN_TOKEN).0, bearer_header(ADMIN_TOKEN).1)
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn agents_pause_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/pause?agency=acme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn agents_pause_succeeds_with_admin_bearer_after_register() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(bearer_header(ADMIN_TOKEN).0, bearer_header(ADMIN_TOKEN).1)
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register.status(), StatusCode::CREATED);

    let pause = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/pause?agency=acme")
                .header(bearer_header(ADMIN_TOKEN).0, bearer_header(ADMIN_TOKEN).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pause.status(), StatusCode::OK);
}

#[tokio::test]
async fn bots_catalog_persist_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/catalog/persist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/catalog/persist")
                .header(bearer_header(ADMIN_TOKEN).0, bearer_header(ADMIN_TOKEN).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn orders_submit_requires_admin_bearer_when_enabled() {
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
        HttpOrderExecutor::dev_accept(),
    );
    let app = build_router(state);
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orders/submit")
                .header("content-type", "application/json")
                .body(Body::from(ORDER_SUBMIT_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn orders_submit_succeeds_with_admin_bearer_when_paper_executor() {
    let _paper_ledger = PaperLedgerTestGuard::acquire();
    PaperLedgerExecutor::clear_ledger();
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
        HttpOrderExecutor::paper(),
    );
    let app = build_router(state);
    let body = r#"{"symbol":"BTC/USDT","side":"buy","quote_amount":5.0,"estimated_daily_loss":0.0,"open_positions":0,"paper_fill_unit_price":100.0,"limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1}}"#;
    let ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orders/submit")
                .header("content-type", "application/json")
                .header(bearer_header(ADMIN_TOKEN).0, bearer_header(ADMIN_TOKEN).1)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    PaperLedgerExecutor::clear_ledger();
}

#[tokio::test]
async fn orders_submit_dev_accept_executor_returns_200() {
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        HttpOrderExecutor::dev_accept(),
    );
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orders/submit")
                .header("content-type", "application/json")
                .body(Body::from(ORDER_SUBMIT_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn orders_submit_live_exchange_reserved_returns_503_with_code() {
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        HttpOrderExecutor::live_exchange_reserved(),
    );
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orders/submit")
                .header("content-type", "application/json")
                .body(Body::from(ORDER_SUBMIT_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "live_exchange_not_wired");
}

#[tokio::test]
async fn orders_submit_live_exchange_wired_returns_200() {
    let _env = EnvTestGuard::acquire();
    std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        HttpOrderExecutor::live_exchange(),
    );
    let app = build_router(state);
    let body = r#"{"symbol":"BTC/USDT","side":"buy","quote_amount":5.0,"estimated_daily_loss":0.0,"open_positions":0,"limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1},"client_order_id":"http-live-wired-cid"}"#;
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
    assert_eq!(response.status(), StatusCode::OK);
    std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
}

#[tokio::test]
async fn orders_reconciliation_poll_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let denied = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orders/reconciliation/poll")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bots_runtime_promote_requires_admin_bearer_when_enabled() {
    use crate::modules::bots::InMemoryBotRuntime;

    let state = ApiState::with_bot_runtime(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
        Arc::new(InMemoryBotRuntime::new()),
    );
    let app = build_router(state);
    let body = r#"{"bot_id":"sma-cross@1:15m:BTCUSDT","promoted_by":"owner-1"}"#;
    let denied = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/runtime/promote")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

async fn json_get(app: &Router, uri: &str) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn agents_resume_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let denied = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/resume?agency=acme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn agents_resume_succeeds_with_admin_bearer_after_pause() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let auth = bearer_header(ADMIN_TOKEN);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(auth.0, auth.1.clone())
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/pause?agency=acme")
                .header(auth.0, auth.1.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let resume = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/resume?agency=acme")
                .header(auth.0, auth.1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resume.status(), StatusCode::OK);
}

#[tokio::test]
async fn agents_retire_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let denied = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/retire?agency=acme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn agents_retire_succeeds_with_admin_bearer_after_register() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let auth = bearer_header(ADMIN_TOKEN);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(auth.0, auth.1.clone())
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    let retire = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/retire?agency=acme")
                .header(auth.0, auth.1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(retire.status(), StatusCode::OK);
}

#[tokio::test]
async fn agents_advisory_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let body = r#"{"agency":"acme","signal":"hold","close":1.0,"candle_timestamp_ms":1}"#;
    let denied = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/advisory")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn agents_advisory_returns_503_with_admin_bearer_when_agent_cannot_consult_jev() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let auth = bearer_header(ADMIN_TOKEN);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(auth.0, auth.1.clone())
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = r#"{"agency":"acme","signal":"hold","close":1.0,"candle_timestamp_ms":1}"#;
    let advisory = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/advisory")
                .header("content-type", "application/json")
                .header(auth.0, auth.1)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(advisory.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn bots_runtime_demote_requires_admin_bearer_when_enabled() {
    use crate::modules::bots::InMemoryBotRuntime;

    let state = ApiState::with_bot_runtime(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
        Arc::new(InMemoryBotRuntime::new()),
    );
    let app = build_router(state);
    let denied = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/runtime/demote")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bots_runtime_promote_and_demote_succeed_with_admin_bearer() {
    use crate::modules::bots::InMemoryBotRuntime;

    let config = Config::default();
    let active_tf = config.market.timeframe.clone();
    let state = ApiState::with_bot_runtime(
        None,
        AppDatabases::empty(),
        None,
        config,
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
        Arc::new(InMemoryBotRuntime::new()),
    );
    let app = build_router(state);
    let auth = bearer_header(ADMIN_TOKEN);
    let persist = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/catalog/persist")
                .header(auth.0, auth.1.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(persist.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(persist.into_body(), usize::MAX)
        .await
        .unwrap();
    let catalog: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let bot_id = catalog["bots"]
        .as_array()
        .and_then(|bots| {
            bots.iter()
                .find(|b| b["timeframe"].as_str() == Some(active_tf.as_str()))
        })
        .and_then(|b| b["bot_id"].as_str())
        .expect("bot_id for active timeframe in persist response");
    let promote_body = format!(r#"{{"bot_id":"{}","promoted_by":"owner-1"}}"#, bot_id);
    let promote = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/runtime/promote")
                .header("content-type", "application/json")
                .header(auth.0, auth.1.clone())
                .body(Body::from(promote_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(promote.status(), StatusCode::OK);
    let demote = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/runtime/demote")
                .header(auth.0, auth.1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(demote.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn orders_reconciliation_poll_succeeds_with_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let auth = bearer_header(ADMIN_TOKEN);
    let ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orders/reconciliation/poll")
                .header(auth.0, auth.1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn monitor_commands_requires_admin_bearer_when_enabled() {
    use crate::modules::monitor::MonitorHandle;

    let (handle, _, _) = MonitorHandle::channel(4, 16);
    let app = build_router(ApiState::with_agent_registry(
        Some(handle),
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
    ));
    let denied = app
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
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn monitor_commands_succeeds_with_admin_bearer_when_handle_present() {
    use crate::modules::monitor::{MonitorCommand, MonitorHandle};

    let (handle, mut commands, _) = MonitorHandle::channel(4, 16);
    let app = build_router(ApiState::with_agent_registry(
        Some(handle),
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
    ));
    let auth = bearer_header(ADMIN_TOKEN);
    let ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/monitor/commands")
                .header("content-type", "application/json")
                .header(auth.0, auth.1)
                .body(Body::from(r#"{"command":"pause"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::ACCEPTED);
    assert!(matches!(commands.recv().await, Some(MonitorCommand::Pause)));
}

#[tokio::test]
async fn meta_includes_http_seams_snapshot() {
    let app = build_router(ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test(ADMIN_TOKEN),
        HttpOrderExecutor::dev_accept(),
    ));
    let meta = json_get(&app, "/api/v1/meta").await;
    assert_eq!(meta["name"], "rust-trading-bot");
    assert!(meta["http_seams"]["http_admin_auth_enabled"].as_bool() == Some(true));
    assert!(meta["http_seams"]["order_execution_mode"].is_string());
}

#[tokio::test]
async fn meta_and_orders_execution_status_agree_on_seams() {
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        HttpOrderExecutor::dev_accept(),
    );
    let app = build_router(state);
    let meta = json_get(&app, "/api/v1/meta").await;
    let exec = json_get(&app, "/api/v1/orders/execution-status").await;
    assert_eq!(meta["http_seams"]["order_execution_mode"], exec["mode"]);
    assert_eq!(
        meta["http_seams"]["live_exchange_wired"],
        exec["live_exchange_wired"]
    );
}

#[tokio::test]
async fn meta_and_bot_runtime_status_agree_on_runtime_enabled() {
    use crate::modules::bots::InMemoryBotRuntime;

    let state = ApiState::with_bot_runtime(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        Arc::new(InMemoryBotRuntime::new()),
    );
    let app = build_router(state);
    let meta = json_get(&app, "/api/v1/meta").await;
    let runtime = json_get(&app, "/api/v1/bots/runtime/status").await;
    assert_eq!(
        meta["http_seams"]["bot_runtime_enabled"],
        runtime["runtime_enabled"]
    );
}

#[tokio::test]
async fn meta_and_orders_execution_status_live_exchange_wired_true() {
    let _env = EnvTestGuard::acquire();
    std::env::set_var("BOT_ORDERS_EXCHANGE_SUBMIT", "recording");
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        HttpOrderExecutor::live_exchange(),
    );
    let app = build_router(state);
    let meta = json_get(&app, "/api/v1/meta").await;
    let exec = json_get(&app, "/api/v1/orders/execution-status").await;
    assert_eq!(meta["http_seams"]["live_exchange_wired"], true);
    assert_eq!(exec["live_exchange_wired"], true);
    std::env::remove_var("BOT_ORDERS_EXCHANGE_SUBMIT");
}

async fn error_body(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn agents_register_accepts_matching_owner_when_bound() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test_with_owner(ADMIN_TOKEN, "owner-1"),
    ));
    let auth = bearer_header(ADMIN_TOKEN);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(auth.0, auth.1)
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn agents_register_rejects_owner_mismatch_when_bound() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test_with_owner(ADMIN_TOKEN, "owner-bound"),
    ));
    let auth = bearer_header(ADMIN_TOKEN);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(auth.0, auth.1)
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = error_body(response).await;
    assert_eq!(body["code"], "owner_mismatch");
}

#[tokio::test]
async fn agents_list_rejects_agency_mismatch_when_bound() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test_with_agency(ADMIN_TOKEN, "bound-agency"),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents?agency=other-agency")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = error_body(response).await;
    assert_eq!(body["code"], "http_agency_mismatch");
}

#[tokio::test]
async fn agents_register_rejects_agency_mismatch_when_bound() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test_with_agency(ADMIN_TOKEN, "bound-agency"),
    ));
    let auth = bearer_header(ADMIN_TOKEN);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .header(auth.0, auth.1)
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = error_body(response).await;
    assert_eq!(body["code"], "http_agency_mismatch");
}

#[tokio::test]
async fn meta_reports_http_bindings_when_configured() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test_with_agency(ADMIN_TOKEN, "agency-1"),
    ));
    let meta = json_get(&app, "/api/v1/meta").await;
    assert_eq!(meta["http_seams"]["http_admin_auth_enabled"], true);
    assert_eq!(meta["http_seams"]["http_agency_binding_active"], true);
    assert_eq!(meta["http_seams"]["http_owner_binding_active"], false);
}

#[tokio::test]
async fn meta_reports_owner_binding_when_configured() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test_with_owner(ADMIN_TOKEN, "owner-1"),
    ));
    let meta = json_get(&app, "/api/v1/meta").await;
    assert_eq!(meta["http_seams"]["http_owner_binding_active"], true);
    assert_eq!(meta["http_seams"]["http_agency_binding_active"], false);
}

#[tokio::test]
async fn meta_reports_agency_binding_without_admin_token() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::for_test_bound_agency("agency-only"),
    ));
    let meta = json_get(&app, "/api/v1/meta").await;
    assert_eq!(meta["http_seams"]["http_admin_auth_enabled"], false);
    assert_eq!(meta["http_seams"]["http_agency_binding_active"], true);
}

#[tokio::test]
async fn orders_submit_paper_executor_returns_200() {
    let _paper_ledger = PaperLedgerTestGuard::acquire();
    PaperLedgerExecutor::clear_ledger();
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        HttpOrderExecutor::paper(),
    );
    let app = build_router(state);
    let body = r#"{"symbol":"BTC/USDT","side":"buy","quote_amount":5.0,"estimated_daily_loss":0.0,"open_positions":0,"paper_fill_unit_price":100.0,"limits":{"max_order_quote":10.0,"max_daily_loss_quote":20.0,"max_open_positions":1}}"#;
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
    assert_eq!(response.status(), StatusCode::OK);
    PaperLedgerExecutor::clear_ledger();
}

#[tokio::test]
async fn bots_runtime_promote_denied_when_bound_agency_without_capable_agent() {
    use crate::modules::bots::InMemoryBotRuntime;

    let config = Config::default();
    let active_tf = config.market.timeframe.clone();
    let state = ApiState::with_bot_runtime(
        None,
        AppDatabases::empty(),
        None,
        config,
        fresh_agents(),
        HttpAdminAuth::for_test_bound_agency("bound-agency"),
        Arc::new(InMemoryBotRuntime::new()),
    );
    let app = build_router(state);
    let register = r#"{"agency":"bound-agency","owner_id":"owner-1","agent_id":"ceo","display_name":"CEO","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"},"consult_jev":false,"promote_runtime_bot":false}"#;
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(register))
                .unwrap(),
        )
        .await
        .unwrap();
    let persist = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/catalog/persist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(persist.status(), StatusCode::OK);
    let catalog: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(persist.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let bot_id = catalog["bots"]
        .as_array()
        .and_then(|bots| {
            bots.iter()
                .find(|b| b["timeframe"].as_str() == Some(active_tf.as_str()))
        })
        .and_then(|b| b["bot_id"].as_str())
        .expect("bot_id");
    let promote_body = format!(r#"{{"bot_id":"{}","promoted_by":"ceo"}}"#, bot_id);
    let denied = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/runtime/promote")
                .header("content-type", "application/json")
                .body(Body::from(promote_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bots_runtime_promote_allowed_when_bound_agency_and_capable_agent() {
    use crate::modules::bots::InMemoryBotRuntime;

    let config = Config::default();
    let active_tf = config.market.timeframe.clone();
    let state = ApiState::with_bot_runtime(
        None,
        AppDatabases::empty(),
        None,
        config,
        fresh_agents(),
        HttpAdminAuth::for_test_bound_agency("bound-agency"),
        Arc::new(InMemoryBotRuntime::new()),
    );
    let app = build_router(state);
    let ceo = r#"{"agency":"bound-agency","owner_id":"owner-1","agent_id":"ceo","display_name":"CEO","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"},"consult_jev":false,"promote_runtime_bot":false}"#;
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(ceo))
                .unwrap(),
        )
        .await
        .unwrap();
    let promoter = r#"{"agency":"bound-agency","owner_id":"owner-1","agent_id":"promoter","display_name":"Promoter","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"},"consult_jev":false,"promote_runtime_bot":true}"#;
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(promoter))
                .unwrap(),
        )
        .await
        .unwrap();
    let persist = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/catalog/persist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let catalog: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(persist.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let bot_id = catalog["bots"]
        .as_array()
        .and_then(|bots| {
            bots.iter()
                .find(|b| b["timeframe"].as_str() == Some(active_tf.as_str()))
        })
        .and_then(|b| b["bot_id"].as_str())
        .expect("bot_id");
    let promote_body = format!(r#"{{"bot_id":"{}","promoted_by":"promoter"}}"#, bot_id);
    let ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bots/runtime/promote")
                .header("content-type", "application/json")
                .body(Body::from(promote_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn portfolio_paper_snapshot_http_reflects_paper_submit() {
    let _paper_ledger = PaperLedgerTestGuard::acquire();
    PaperLedgerExecutor::clear_ledger();
    let state = ApiState::with_order_executor(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
        HttpOrderExecutor::paper(),
    );
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
    let json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(snapshot.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(json["available"], "900");
    assert_eq!(json["positions"].as_array().map(|a| a.len()), Some(1));
    PaperLedgerExecutor::clear_ledger();
}

#[tokio::test]
async fn bots_catalog_http_lists_monitor_registry_v2_periods() {
    use crate::modules::http_bridge::config::MonitorStrategyConfigEntry;

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
    config.validate().expect("monitor_registry v2 config valid");
    let state = ApiState::new(None, AppDatabases::empty(), None, config);
    let prelude = state
        .bot_catalog_for_config()
        .expect("bot_catalog_for_config before HTTP");
    assert!(
        prelude.bots.iter().any(|entry| entry.strategy_version == 2),
        "domain catalog missing v2: versions={:?}",
        prelude
            .bots
            .iter()
            .map(|entry| entry.strategy_version)
            .collect::<Vec<_>>()
    );
    let app = build_router(state);
    let catalog = json_get(&app, "/api/v1/bots/catalog").await;
    let v2 = catalog["bots"]
        .as_array()
        .and_then(|bots| {
            bots.iter()
                .find(|b| b.get("strategy_version").and_then(|v| v.as_u64()) == Some(2))
        })
        .expect("strategy_version 2 in GET /bots/catalog");
    assert_eq!(v2["monitor_evaluator"], "sma_cross");
    assert_eq!(v2["monitor_fast_period"], 3);
    assert_eq!(v2["monitor_slow_period"], 15);
}

#[tokio::test]
async fn agents_register_promote_runtime_bot_visible_via_http_get() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
    ));
    let register = r#"{"agency":"cap-agency","owner_id":"owner-1","agent_id":"promoter","display_name":"Promoter","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"},"consult_jev":true,"promote_runtime_bot":true}"#;
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(register))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let get = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents/promoter?agency=cap-agency")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let agent: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(get.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(agent["promote_runtime_bot"], true);
    assert_eq!(agent["consult_jev"], true);
}

#[tokio::test]
async fn agents_audit_lists_lifecycle_events_after_register_and_pause() {
    let app = build_router(ApiState::with_agent_registry(
        None,
        AppDatabases::empty(),
        None,
        Config::default(),
        fresh_agents(),
        HttpAdminAuth::disabled(),
    ));
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(AGENT_REGISTER_JSON))
                .unwrap(),
        )
        .await
        .unwrap();
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents/ceo/pause?agency=acme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let audit = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents/audit?agency=acme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(audit.status(), StatusCode::OK);
    let log: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(audit.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let kinds = log["events"]
        .as_array()
        .expect("events")
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect::<Vec<_>>();
    assert!(kinds.iter().any(|k| k.contains("Registered")));
    assert!(kinds.iter().any(|k| k.contains("Paused")));
}

#[tokio::test]
async fn provider_credentials_admin_list_returns_501_with_admin_bearer() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/provider-credentials")
                .header("authorization", format!("Bearer {}", ADMIN_TOKEN))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn provider_credentials_admin_list_requires_admin_bearer_when_enabled() {
    let app = router_with_admin(HttpAdminAuth::for_test(ADMIN_TOKEN));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/provider-credentials")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
