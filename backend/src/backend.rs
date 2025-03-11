use std::sync::Arc;

use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_s3::{config::Credentials, primitives::ByteStreamError};
use bytes::Bytes;
use diesel::prelude::*;
use itertools::Itertools;
use recognition::{plantnet::PlantNetRecogniser, PlantRecogniser};
use tokio::sync::Mutex;
use tracing::debug;
use uuid::Uuid;

use crate::{config::AppConfig, models::*, schema::listings};

pub mod recognition;

#[derive(Clone)]
pub struct Backend<P: PlantRecogniser = PlantNetRecogniser> {
    pub db: Arc<Mutex<PgConnection>>,
    pub s3_client: aws_sdk_s3::Client,
    pub images_bucket: String,
    pub plant_recognition: Arc<P>,
}

impl<P: PlantRecogniser> Backend<P> {
    pub async fn new(config:  &AppConfig) -> Self {
        let con = PgConnection::establish(config.database_url())
                .unwrap_or_else(|err| panic!("Error connecting to {}, error: {err}", config.database_url()));

        let db = Arc::new(Mutex::new(con));

        let s3_client = create_s3_client(
            config.s3_access_key(),
            config.s3_secret_key(),
            config.s3_endpoint(),
            config.s3_images_bucket()
        ).await;

        let images_bucket = config.s3_images_bucket().to_owned();

        // create bucket if it doesn't exist
        let head_bucket_response = s3_client.head_bucket().bucket(&images_bucket)
            .send().await;
        if head_bucket_response.is_err_and(|e| e.into_service_error().is_not_found()) {
            s3_client.create_bucket()
                .bucket(&images_bucket)
                .send().await.unwrap();
        }

        let plant_recognition = Arc::new(P::new(config));

        Backend { db, s3_client, images_bucket, plant_recognition }
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

    pub async fn create_listing(&self, mut listing: InsertListing) -> BackendResult<Listing> {
        use crate::schema::{users, listings};

        listing.title = listing
            .title
            .split_whitespace()
            .map(|word| word.trim())
            .join(" ");

        let mut con = self.db.lock().await;

        let user_exists: i64 = users::table.find(listing.author)
            .filter(users::location.is_not_null())
            .count()
            .get_result(&mut *con).optional()?
            .unwrap_or_default();

        if user_exists != 1 {
            return Err(BackendError::ListingHasNoLocation);
        }

        listing.insert_into(listings::table)
            .returning(Listing::as_select())
            .get_result(&mut *con)
            .map_err(Into::into)
    }

    pub async fn update_listing(&self, listing_update: &ListingUpdate) -> BackendResult<Option<Listing>> {
        use crate::schema::listings;
        if listing_update.id.is_none() {
            return Err(BackendError::ListingUpdateMissingId);
        }

        let mut con = self.db.lock().await;

        diesel::update(listings::table)
            .set(listing_update)
            .returning(Listing::as_select())
            .get_result(&mut *con).optional()
            .map_err(Into::into)
    }

    pub async fn get_listing(&self, listing_id: Uuid) -> BackendResult<Option<Listing>> {
        use crate::schema::listings::dsl::*;

        let mut con = self.db.lock().await;

        let listing = listings.find(listing_id)
            .select(Listing::as_select())
            .get_result(&mut *con).optional()?;

        Ok(listing)
    }

    pub async fn delete_listing(&self, listing_id: Uuid) -> BackendResult<Option<Listing>> {
        use crate::schema::listings::dsl::*;

        let mut con = self.db.lock().await;

        let listing = diesel::delete(listings.find(listing_id))
            .returning(Listing::as_returning())
            .get_result(&mut *con).optional()?;

        Ok(listing)
    }

    #[allow(dead_code)] // currently used, but only in tests
    pub async fn delete_all(&self) -> BackendResult<()> {
        let mut con = self.db.lock().await;

        diesel::delete(listings::table)
            .execute(&mut *con)?;

        self.s3_client.delete_bucket()
            .bucket(&self.images_bucket)
            .send().await.map_err(Into::<aws_sdk_s3::Error>::into)?;

        Ok(())
    }

    pub async fn upload_image(&self, user: Uuid, image: Bytes) -> BackendResult<Uuid> {
        let file_key = Uuid::now_v7();

        let client = &self.s3_client;

        debug!(self.images_bucket, key=?file_key, input_len=image.len(), "Uploading image to s3");

        client.put_object()
            .bucket(&self.images_bucket)
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

        Ok(file_key)
    }

    /// On success, returns a tuple of contenttype and bytes.
    pub async fn get_image(&self, image: Uuid) -> BackendResult<Option<(String, Bytes)>> {
        let result = self.s3_client.get_object()
            .bucket(&self.images_bucket)
            .key(image)
            .send()
            .await;

        match result {
            Err(err) if err.as_service_error().is_some_and(|e| e.is_no_such_key()) => {
                Ok(None)
            }
            Err(err) => Err(BackendError::S3(aws_sdk_s3::Error::from(err))),
            Ok(result) => {
                let bytes = result.body.collect().await?.into_bytes();
                Ok(Some((result.content_type.unwrap_or("image/jpeg".to_string()), bytes)))
            }
        }
    }
}

async fn create_s3_client(access_key: &str, secret_key: &str, endpoint: &str, bucket: &str) -> aws_sdk_s3::Client {
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let credentials_provider = Credentials::new(access_key, secret_key, None, None, "Environment");
    let s3_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(endpoint)
        .credentials_provider(credentials_provider)
        .load().await;

    let s3_config: aws_sdk_s3::Config = (&s3_config).into();
    let s3_config = s3_config.to_builder()
        .force_path_style(true)
        .build();

    let client = aws_sdk_s3::Client::from_conf(s3_config);

    let head_bucket = client.head_bucket()
        .bucket(bucket)
        .send().await;

    if head_bucket.is_err_and(
        |err| err.into_service_error().is_not_found()
    ) {
        client.create_bucket()
            .bucket(bucket)
            .send().await.unwrap();
    }

    return client;
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {

    #[error("Listing's owner has no location")]
    ListingHasNoLocation,

    #[error("Listing update has no id!")]
    ListingUpdateMissingId,

    #[error("DB error: {0}")]
    Db(#[from] diesel::result::Error),

    #[error("S3 error: {0}")]
    S3(#[from] aws_sdk_s3::Error),

    #[error("S3 bytestream error: {0}")]
    S3Bytestream(#[from] ByteStreamError),

    #[error("Tokio join error")]
    TokioJoinError(#[from] tokio::task::JoinError),
}

pub type BackendResult<T> = Result<T, BackendError>;

#[cfg(test)]
mod tests {
    use std::{error::Error, sync::Arc};

    use diesel::{Connection as _, PgConnection};
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness as _};
    use reqwest::Url;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use crate::models::{InsertListing, ListingType};

    use super::{create_s3_client, recognition::plantnet::PlantNetRecogniser, Backend};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

    async fn setup_test_backend() -> Backend {
        let db_con = PgConnection::establish("postgres://postgres:postgres@127.0.0.1:5432/postgres")
            .unwrap();

        let plant_recognition = PlantNetRecogniser::from_parts(
            Url::parse("https://example.net").unwrap(),
            "fake_api_key".to_string()
        );

        let s3_client = create_s3_client("root", "rootpassword", "http://localhost:9000/", "images").await;
        let backend = Backend {
            db: Arc::new(Mutex::new(db_con)),
            s3_client,
            images_bucket: "images".to_string(),
            plant_recognition: Arc::new(plant_recognition),
        };

        {
            let mut con = backend.db.lock().await;
            con.run_pending_migrations(MIGRATIONS).unwrap();
        }

        backend.delete_all().await.unwrap();

        {
            let mut con = backend.db.lock().await;
            con.begin_test_transaction().unwrap();
        }

        backend
    }

    #[tokio::test]
    async fn insert_test() -> Result<(), Box<dyn Error>> {
        let backend = setup_test_backend().await;

        let new_listing = InsertListing {
            title: "Monstera".to_string(),
            description: "cool plant".to_string(),
            author: Uuid::new_v4(),
            listing_type: ListingType::Selling,
            tradeable: Some(false),
            thumbnail: Uuid::now_v7()
        };

        backend.create_listing(new_listing).await?;

        let listings = backend.get_all_listings().await?;
        assert_eq!(listings.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn insert_test_two() -> Result<(), Box<dyn Error>> {
        let backend = setup_test_backend().await;

        let new_listing = InsertListing {
            title: "Non-Monstera".to_string(),
            description: "not so cool plant".to_string(),
            author: Uuid::new_v4(),
            listing_type: ListingType::Buying,
            tradeable: Some(true),
            thumbnail: Uuid::now_v7()
        };

        backend.create_listing(new_listing).await?;

        let listings = backend.get_all_listings().await?;
        assert_eq!(listings.len(), 1);

        Ok(())
    }
}
