use std::sync::Arc;

use axum::{extract::{Multipart, Path, State}, http::StatusCode, routing::*, Json};
use diesel::prelude::*;
use serde::Serialize;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

use crate::{models::*, AppState};

/// TODO: Update
pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/listing", get(get_all_listings).post(create_listing))
        .route("/listing/:id", get(get_listing))
        .route("/image", post(upload_image))
}

async fn get_all_listings(State(db): State<Arc<Mutex<PgConnection>>>) -> Result<Json<Vec<Listing>>, StatusCode> {
    use crate::schema::listings::dsl::*;

    let mut con = db.lock().await;

    listings
        .limit(100)
        .select(Listing::as_select())
        .load(&mut *con)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// TODO: Get author from auth
async fn create_listing(State(db): State<Arc<Mutex<PgConnection>>>, Json(body): Json<InsertListing>) -> Result<Json<Listing>, StatusCode> {
    use crate::schema::listings::dsl::*;

    let mut con = db.lock().await;

    body.insert_into(listings)
        .returning(Listing::as_select())
        .get_result(&mut *con)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_listing(State(db): State<Arc<Mutex<PgConnection>>>, Path(listing_id): Path<i32>) -> Result<Json<Listing>, StatusCode> {
    use crate::schema::listings::dsl::*;

    let mut con = db.lock().await;

    listings.find(listing_id)
        .select(Listing::as_select())
        .get_result(&mut *con).optional()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::BAD_REQUEST)
}

#[derive(Serialize)]
struct UploadImageResponse {
    file_key: Uuid,
}

async fn upload_image(State(state): State<AppState>, mut multipart: Multipart) -> Result<Json<UploadImageResponse>, (StatusCode, String)> {
    let client = state.s3_client;
    let bucket = state.config.s3_images_bucket();

    let file_key = Uuid::now_v7();

    let image;

    if let Some(field) = multipart.next_field().await
        .map_err(|err|
            (StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error parsing multipart form data: {err:?}")))?
    {
        if field.content_type() != Some("image/jpeg") {
            return Err((StatusCode::BAD_REQUEST, "Invalid content type on multipart field, should be 'image/jpeg'".to_string()));
        }
        image = field.bytes().await
            .map_err(|err|
                (StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error parsing field in multipart form: {err:?}")))?;
    } else {
        return Err((StatusCode::BAD_REQUEST, "Zero fields in multipart request".to_string()));
    }

    info!(bucket, key=?file_key, input_len=image.len(), "Uploading image to s3");

    client.put_object()
        .bucket(bucket)
        .key(file_key)
        .body(image.into())
        .content_type("image/jpeg")
        .send()
        .await
        .map_err(|err|
            (StatusCode::INTERNAL_SERVER_ERROR,
                format!("Couldn't upload file to S3 server: {err:?}"))
            )?;

    let new_image = InsertImage {
        file_key,
        uploaded_by_user: None // TODO: Put user here
    };

    let mut con = state.db.lock().await;

    new_image.insert_into(crate::schema::images::table)
        .execute(&mut *con)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    Ok(Json(UploadImageResponse { file_key }))
}

