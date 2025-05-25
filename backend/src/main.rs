use std::sync::Arc;

use auth::{initialize_auth, AuthState};
use axum::{extract::FromRef, middleware, response::Redirect, routing::get, Router};
use backend::Backend;
use config::AppConfig;
use tower_http::{services::ServeDir, ServiceBuilderExt};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod frontend;
mod backend;
mod models;
mod schema;
mod config;
mod auth;
mod rest;

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

    let config = AppConfig::new();

    let backend = Backend::new(&config).await;

    let auth_state = initialize_auth(&config).await;

    let global_state = AppState { backend, auth_state, config: Arc::new(config) };

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
                .compression()
                .decompression()
                .request_body_limit(10 * 1024 * 1024 /* 10MB */)
                .trace_for_http()
        );

    #[cfg(debug_assertions)]
    let socket_address = "localhost:3000";

    #[cfg(not(debug_assertions))]
    let socket_address = "0.0.0.0:3000";

    info!("Listening on http://{socket_address}/");

    let listener = TcpListener::bind(socket_address).await.unwrap();

    #[cfg(debug_assertions)]
    warn!("Running in debug mode (non-secure auth cookies, e.g.)");

    axum::serve(listener, app).await.unwrap();
}
