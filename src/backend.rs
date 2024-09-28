use std::sync::Arc;

use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::config::Credentials;
use bytes::Bytes;
use diesel::prelude::*;
use tokio::sync::Mutex;
use tracing::debug;
use uuid::Uuid;

use crate::{config::AppConfig, models::*};

#[derive(Clone)]
pub struct Backend {
    pub db: Arc<Mutex<PgConnection>>,
    pub s3_client: aws_sdk_s3::Client,
}

impl Backend {
    pub async fn new(config:  &AppConfig) -> Self {
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

        let s3_client = aws_sdk_s3::Client::from_conf(s3_config);

        Backend { db, s3_client }
    }

    pub async fn get_all_listings(&self) -> BackendResult<Vec<Listing>> {
        use crate::schema::listings::dsl::*;

        let mut con = self.db.lock().await;

        listings
            .limit(100)
            .select(Listing::as_select())
            .load(&mut *con)
            .map_err(Into::into)
    }

    pub async fn create_listing(&self, listing: InsertListing) -> BackendResult<Listing> {
        use crate::schema::listings::dsl::*;

        let mut con = self.db.lock().await;

        listing.insert_into(listings)
            .returning(Listing::as_select())
            .get_result(&mut *con)
            .map_err(Into::into)
    }

    pub async fn get_listing(&self, listing_id: i32) -> BackendResult<Option<Listing>> {
        use crate::schema::listings::dsl::*;

        let mut con = self.db.lock().await;

        let listing = listings.find(listing_id)
            .select(Listing::as_select())
            .get_result(&mut *con).optional()?;

        Ok(listing)
    }

    pub async fn upload_image(&self, user: Uuid, bucket: &str, file_key: Uuid, image: Bytes) -> BackendResult<()> {
        let client = &self.s3_client;

        debug!(bucket, key=?file_key, input_len=image.len(), "Uploading image to s3");

        client.put_object()
            .bucket(bucket)
            .key(file_key)
            .body(image.into())
            .content_type("image/jpeg")
            .send()
            .await.map_err(Into::<aws_sdk_s3::Error>::into)?;

        let new_image = InsertImage {
            file_key,
            uploaded_by_user: Some(user)
        };

        let mut con = self.db.lock().await;

        new_image.insert_into(crate::schema::images::table)
            .execute(&mut *con)?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("DB error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("S3 error: {0}")]
    S3(#[from] aws_sdk_s3::Error)
}

pub type BackendResult<T> = Result<T, BackendError>;
