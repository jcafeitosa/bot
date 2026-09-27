use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::modules::config_api::Config;
use crate::core::persistence::Database;
use crate::core::providers::JevAdvisor;
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
            paths >= 24,
            "expected expanded openapi surface, got {paths}"
        );
        assert!(doc["paths"]["/api/v1/agents"].is_object());
        assert!(doc["paths"]["/api/v1/risk/gate-signal"].is_object());
    }
}
