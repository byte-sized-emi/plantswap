use std::{env, sync::Arc};

use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::{config::Credentials, Client};
use axum::{extract::{DefaultBodyLimit, FromRef}, response::Redirect, routing::get, Router};
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

#[derive(Clone, FromRef)]
struct AppState {
    pub db: Arc<Mutex<PgConnection>>,
    pub s3_client: Client,
    pub s3_images_bucket: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("PLANTS_LOG"))
        .init();

    let database_url = env::var("PLANTS_DATABASE_URL").expect("PLANTS_DATABASE_URL must be set");

    let con = PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    let db = Arc::new(Mutex::new(con));

    let s3_endpoint = env::var("PLANTS_S3_ENDPOINT").expect("PLANTS_S3_ENDPOINT must be set");
    let s3_access_key = env::var("PLANTS_S3_ACCESS_KEY").expect("PLANTS_S3_ACCESS_KEY must be set");
    let s3_secret_key = env::var("PLANTS_S3_SECRET_KEY").expect("PLANTS_S3_SECRET_KEY must be set");

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(s3_endpoint)
        .credentials_provider(Credentials::new(s3_access_key, s3_secret_key, None, None, "Environment"))
        .load().await;

    let s3_config: aws_sdk_s3::Config = (&config).into();
    let s3_config = s3_config.to_builder()
        .force_path_style(true)
        .build();

    let s3_client = Client::from_conf(s3_config);

    let s3_images_bucket = env::var("PLANTS_S3_IMAGES_BUCKET").expect("PLANTS_S3_IMAGES_BUCKET must be set");

    let global_state = AppState { db, s3_client, s3_images_bucket };

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/home") }))
        .route("/home", get(render_homepage))
        .route("/about", get(render_about))
        .route("/ping", get(|| async { "Pong" }))
        .nest("/api/v1/", backend::router())
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(global_state)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024 /* 10MB */));

    let socket_address = "0.0.0.0:3000";
    info!("Listening on http://{socket_address}/");

    let listener = TcpListener::bind(socket_address).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
