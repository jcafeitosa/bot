use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::core::persistence::Database;
use crate::core::providers::JevAdvisor;
use crate::modules::config_api::Config;
use crate::presentation::http::agent_log::agent_debug_log;
use crate::presentation::http::openapi::ApiDoc;
use crate::presentation::http::routes;
use crate::presentation::http::state::ApiState;

pub async fn run(bind: SocketAddr, app_config: Config) -> anyhow::Result<()> {
    let database = match std::env::var("DATABASE_URL") {
        Ok(url) => match Database::connect_from_url(&url).await {
            Ok(db) => Some(db),
            Err(error) => {
                tracing::warn!(target: "api", %error, "DATABASE_URL present but connection failed; readyz will report degraded database checks");
                None
            }
        },
        Err(_) => None,
    };

    let jev = JevAdvisor::from_env(app_config.jev.clone()).ok().flatten();
    let state = ApiState::new(None, database, jev, app_config);
    let app = build_router(state);

    agent_debug_log(
        "B",
        "presentation/http/server.rs:run",
        "HTTP router mounted",
        serde_json::json!({
            "bind": bind.to_string(),
            "openapi_paths": ApiDoc::openapi().paths.paths.len(),
        }),
        "post-expand",
    );

    let listener = TcpListener::bind(bind).await?;
    tracing::info!(target: "api", %bind, "HTTP API listening (Scalar at /docs)");
    axum::serve(listener, app).await?;
    Ok(())
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
            paths >= 25,
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
        let posts: [(&str, &str); 5] = [
            (
                "/api/v1/risk/profile-limits",
                r#"{"profile":"moderate","base":{"max_order_quote":100.0,"max_daily_loss_quote":50.0,"max_open_positions":3}}"#,
            ),
            (
                "/api/v1/risk/validate-intent",
                r#"{"intent":{"quote_amount":10.0,"estimated_daily_loss":0.0,"open_positions":0},"limits":{"max_order_quote":100.0,"max_daily_loss_quote":50.0,"max_open_positions":3}}"#,
            ),
            (
                "/api/v1/risk/gate-signal",
                r#"{"signal":"hold","limits":{"max_order_quote":100.0,"max_daily_loss_quote":50.0,"max_open_positions":3},"run_mode":"paper"}"#,
            ),
            (
                "/api/v1/strategy/evaluate-sma",
                r#"{"fast_period":2,"slow_period":3,"candles":[{"timestamp_ms":0,"open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":100.0},{"timestamp_ms":60000,"open":1.5,"high":2.5,"low":1.0,"close":2.0,"volume":110.0},{"timestamp_ms":120000,"open":2.0,"high":3.0,"low":1.5,"close":2.5,"volume":120.0}]}"#,
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
            assert!(
                response.status().is_success(),
                "unexpected status {} for {path}",
                response.status()
            );
        }
    }

    #[test]
    fn openapi_surface_lists_core_paths() {
        let doc = ApiDoc::openapi();
        let paths = &doc.paths.paths;
        assert_eq!(paths.len(), 25, "update test when adding utoipa paths");
        for key in [
            "/api/v1/config/active",
            "/api/v1/agents/{agent_id}/advisory",
            "/api/v1/monitor/commands",
            "/api/v1/risk/gate-signal",
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

        let app = build_router(ApiState::new(Some(handle), None, None, Config::default()));

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
        let register_body = r#"{"agency":"acme","owner_id":"owner-1","agent_id":"ceo","display_name":"CEO","role":"ceo","supervisor":{"kind":"owner","owner_id":"owner-1"},"consult_jev":true}"#;
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
                    .uri("/api/v1/agents/ceo?agency=acme")
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
                    .uri("/api/v1/agents/ceo/pause?agency=acme")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pause.status(), StatusCode::OK);

        let resume = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/agents/ceo/resume?agency=acme")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resume.status(), StatusCode::OK);

        let advisory = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/agents/ceo/advisory")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"agency":"acme","signal":"hold","close":1.0,"candle_timestamp_ms":0}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(advisory.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
