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
use crate::modules::orders::PaperLedgerExecutor;
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
