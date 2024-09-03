use serde::Deserialize;
use config::{Config, FileFormat};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    database_url: String,
    s3_endpoint: String,
    s3_access_key: String,
    s3_secret_key: String,
    s3_images_bucket: String,
}

impl AppConfig {
    /// Read config from env's and a config file
    pub fn new() -> AppConfig {
        Config::builder()
            .add_source(config::File::new("config", FileFormat::Toml).required(false))
            .add_source(config::Environment::with_prefix("PLANTS"))
            .build().expect("Building config went wrong")
            .try_deserialize().expect("Config is wrong")
    }

    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn s3_endpoint(&self) -> &str {
        &self.s3_endpoint
    }

    pub fn s3_access_key(&self) -> &str {
        &self.s3_access_key
    }

    pub fn s3_secret_key(&self) -> &str {
        &self.s3_secret_key
    }

    pub fn s3_images_bucket(&self) -> &str {
        &self.s3_images_bucket
    }
}
