use serde::Deserialize;
use config::{Config, FileFormat};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    base_url: String,
    database_url: String,
    s3_endpoint: String,
    s3_access_key: String,
    s3_secret_key: String,
    s3_images_bucket: String,
    auth_server_url: String,
    auth_admin_role: String,
    auth_client_id: String,
    redis_url: String,
    plantnet_api_key: String,
    plantnet_api_url: String,
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

    pub fn base_url(&self) -> &str {
        &self.base_url
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

    pub fn auth_server_url(&self) -> &str {
        &self.auth_server_url
    }

    pub fn auth_admin_role(&self) -> &str {
        &self.auth_admin_role
    }

    pub fn auth_client_id(&self) -> &str {
        &self.auth_client_id
    }

    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }

    pub fn plantnet_api_key(&self) -> &str {
        &self.plantnet_api_key
    }

    /// Plantnet api url, normally https://my-api.plantnet.org/v2/
    pub fn plantnet_api_url(&self) -> &str {
        &self.plantnet_api_url
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}
