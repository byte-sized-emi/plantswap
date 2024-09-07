use std::sync::Arc;

use auth::initialize_auth;
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::{config::Credentials, Client};
use axum::{extract::{DefaultBodyLimit, FromRef}, response::Redirect, routing::get, Router};
use axum_login::{tower_sessions::{CachingSessionStore, SessionManagerLayer}, AuthManagerLayerBuilder};
use config::AppConfig;
use diesel::{Connection, PgConnection};
use tower_http::services::ServeDir;
use tokio::{net::TcpListener, sync::Mutex};
use tower_sessions_moka_store::MokaStore;
use tower_sessions_redis_store::{fred::{prelude::*, types::RedisConfig}, RedisStore};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod frontend;
mod backend;
mod models;
mod schema;
mod config;
mod auth;

#[derive(Clone, FromRef)]
struct AppState {
    pub db: Arc<Mutex<PgConnection>>,
    pub s3_client: Client,
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("PLANTS_LOG"))
        .init();

    let config = AppConfig::new();

    let con = PgConnection::establish(config.database_url())
            .unwrap_or_else(|_| panic!("Error connecting to {}", config.database_url()));

    let db = Arc::new(Mutex::new(con));

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let credentials_provider = Credentials::new(config.s3_access_key(), config.s3_secret_key(), None, None, "Environment");
    let s3_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(config.s3_endpoint())
        .credentials_provider(credentials_provider)
        .load().await;

    let s3_config: aws_sdk_s3::Config = (&s3_config).into();
    let s3_config = s3_config.to_builder()
        .force_path_style(true)
        .build();

    let s3_client = Client::from_conf(s3_config);

    let auth_state = initialize_auth(&config, db.clone()).await;

    let moka_store = MokaStore::new(Some(2_000));
    let redis_config = RedisConfig::from_url(config.redis_url()).unwrap();
    let pool = RedisPool::new(redis_config, None, None, None, 4).unwrap();

    let _redis_conn = pool.connect();
    pool.wait_for_connect().await.unwrap();

    let redis_store = RedisStore::new(pool);
    let caching_store = CachingSessionStore::new(moka_store, redis_store);

    let session_layer = SessionManagerLayer::new(caching_store);
    let auth_layer = AuthManagerLayerBuilder::new(auth_state, session_layer).build();

    let global_state = AppState { db, s3_client, config: Arc::new(config) };

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/home") }))
        .merge(frontend::router())
        .route("/ping", get(|| async { "Pong" }))
        .nest("/auth", auth::router())
        .nest("/api/v1/", backend::router())
        .with_state(global_state)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024 /* 10MB */))
        .layer(auth_layer)
        .nest_service("/assets", ServeDir::new("assets"));

    let socket_address = "0.0.0.0:3000";
    info!("Listening on http://{socket_address}/");

    let listener = TcpListener::bind(socket_address).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
