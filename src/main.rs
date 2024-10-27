use std::sync::Arc;

use auth::initialize_auth;
use axum::{extract::{DefaultBodyLimit, FromRef}, response::Redirect, routing::get, Router};
use axum_login::{tower_sessions::{CachingSessionStore, Expiry, SessionManagerLayer}, AuthManagerLayerBuilder};
use backend::Backend;
use config::AppConfig;
use tower_http::services::ServeDir;
use tokio::net::TcpListener;
use tower_sessions_moka_store::MokaStore;
use tower_sessions_redis_store::{fred::{prelude::*, types::RedisConfig}, RedisStore};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod frontend;
mod backend;
mod models;
mod schema;
mod config;
mod auth;

#[derive(Clone, FromRef)]
struct AppState {
    pub backend: Backend,
    pub config: Arc<AppConfig>,
}

const LOGIN_URL: &str = "/auth/login";

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("PLANTS_LOG"))
        .init();

    let config = AppConfig::new();

    let backend = Backend::new(&config).await;

    let auth_state = initialize_auth(&config, backend.clone()).await;

    let moka_store = MokaStore::new(Some(2_000));
    let redis_config = RedisConfig::from_url(config.redis_url()).unwrap();
    let pool = RedisPool::new(redis_config, None, None, None, 4).unwrap();

    let _redis_conn = pool.connect();
    pool.wait_for_connect().await.unwrap();

    let redis_store = RedisStore::new(pool);
    let caching_store = CachingSessionStore::new(moka_store, redis_store);

    let session_layer = SessionManagerLayer::new(caching_store)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(100)));

    #[cfg(debug_assertions)]
    let session_layer = session_layer.with_secure(false);

    let auth_layer = AuthManagerLayerBuilder::new(auth_state, session_layer).build();

    let global_state = AppState { backend, config: Arc::new(config) };

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/home") }))
        .merge(frontend::router())
        .route("/ping", get(|| async { "Pong" }))
        .nest("/auth", auth::router())
        .with_state(global_state)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024 /* 10MB */))
        .layer(auth_layer)
        .nest_service("/assets", ServeDir::new("assets"));

    let socket_address = "localhost:3000";
    info!("Listening on http://{socket_address}/");

    let listener = TcpListener::bind(socket_address).await.unwrap();

    #[cfg(debug_assertions)]
    warn!("Running in debug mode (non-secure auth cookies, e.g.)");

    axum::serve(listener, app).await.unwrap();
}
