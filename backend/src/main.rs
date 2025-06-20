use std::sync::Arc;

use auth::{AuthState, initialize_auth};
use axum::{body::Body, extract::FromRef, http::{HeaderName, Request}, middleware, response::Redirect, routing::get, Router};
use backend::Backend;
use config::AppConfig;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{body::Limited, request_id::{MakeRequestId, RequestId}, services::ServeDir, trace::TraceLayer, ServiceBuilderExt};
use tracing::{info, info_span, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod auth;
mod backend;
mod config;
mod frontend;
mod models;
mod rest;
mod schema;

#[derive(Clone, FromRef)]
struct AppState {
    pub backend: Backend,
    pub auth_state: AuthState,
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env.local").ok();
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("PLANTS_LOG"))
        .init();

    let config = Arc::new(AppConfig::new());

    let backend = Backend::new(&config).await;

    let auth_state = initialize_auth(&config).await;

    let global_state = AppState {
        backend,
        auth_state: auth_state.clone(),
        config: config.clone(),
    };

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/home") }))
        .merge(frontend::router())
        .nest("/api/v1", rest::router())
        .route("/ping", get(|| async { "Pong" }))
        .nest("/auth", auth::router())
        .nest_service("/assets", ServeDir::new("assets"))
        .fallback(frontend::fallback_handler)
        .with_state(global_state)
        .layer(middleware::from_fn_with_state(auth_state, auth::base))
        .layer(
            ServiceBuilder::new()
                .request_body_limit(10 * 1024 * 1024 /* 10MiB */)
                .sensitive_headers([HeaderName::from_static("authorization")])
                .compression()
                .decompression()
                .catch_panic()
                .set_x_request_id(UuidRequestId)
                .propagate_x_request_id()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|req: &Request<Limited<Body>>| {
                            let request_id = req.headers()
                                .get(HeaderName::from_static("x-request-id"))
                                .map(|h| h.to_str().ok())
                                .flatten()
                                .unwrap_or("<unknown>");

                            info_span!(
                                "request",
                                method = %req.method(),
                                uri = %req.uri(),
                                request_id,
                            )
                        })
                ),
        );

    #[cfg(debug_assertions)]
    let socket_address = "localhost:3000";

    #[cfg(not(debug_assertions))]
    let socket_address = "0.0.0.0:3000";

    info!("Listening on http://{socket_address}/, server is accessible under {}", config.base_url());

    let listener = TcpListener::bind(socket_address).await.unwrap();

    #[cfg(debug_assertions)]
    warn!("Running in debug mode");

    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
struct UuidRequestId;

impl MakeRequestId for UuidRequestId {
    fn make_request_id<B>(&mut self, _: &axum::http::Request<B>) -> Option<tower_http::request_id::RequestId> {
        let uuid = Uuid::new_v4().to_string();
        Some(RequestId::new(uuid.parse().unwrap()))
    }
}
