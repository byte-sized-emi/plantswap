use std::sync::Arc;

use auth::{initialize_auth, AuthState};
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::{config::Credentials, Client};
use axum::{extract::{DefaultBodyLimit, FromRef}, response::Redirect, routing::get, Router};
use config::AppConfig;
use diesel::{Connection, PgConnection};
use frontend::{render_about, render_homepage};
use tower_http::services::ServeDir;
use tokio::{net::TcpListener, sync::Mutex};
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
    pub auth_state: Arc<AuthState>,
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

    let auth_state = Arc::new(initialize_auth(&config).await);

    let global_state = AppState { db, s3_client, config: Arc::new(config), auth_state };

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/home") }))
        .route("/home", get(render_homepage))
        .route("/about", get(render_about))
        .route("/ping", get(|| async { "Pong" }))
        .nest("/auth", auth::router())
        .nest("/api/v1/", backend::router())
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(global_state)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024 /* 10MB */));

    let socket_address = "0.0.0.0:3000";
    info!("Listening on http://{socket_address}/");

    let listener = TcpListener::bind(socket_address).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
