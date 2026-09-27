use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::core::persistence::Database;
use crate::presentation::http::agent_log::agent_debug_log;
use crate::presentation::http::openapi::ApiDoc;
use crate::presentation::http::routes;
use crate::presentation::http::state::ApiState;

pub async fn run(bind: SocketAddr) -> anyhow::Result<()> {
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

    let state = ApiState::new(None, database);
    let app = build_router(state);

    agent_debug_log(
        "B",
        "presentation/http/server.rs:run",
        "HTTP router mounted",
        serde_json::json!({
            "bind": bind.to_string(),
            "routes": [
                "/healthz",
                "/readyz",
                "/openapi.json",
                "/docs",
                "/api/v1/meta"
            ]
        }),
        "pre-fix",
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
}
