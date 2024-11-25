use askama_axum::IntoResponse;
use axum::{extract::{Path, State}, http::StatusCode, routing::{get, post}, Json, Router};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use crate::{auth::AuthSession, backend::Backend, models::{InsertListing, ListingType, ListingUpdate}, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/listing", post(create_listing).get(get_all_listings))
        .route("/listing/:id", get(get_listing)
            .put(update_listing).patch(update_listing))
}

#[derive(Deserialize)]
struct InsertListingBody {
    pub title: String,
    pub description: String,
    pub listing_type: ListingType,
    pub pictures: Vec<Uuid>,
    pub thumbnail: Uuid,
    pub tradeable: bool,
}

impl InsertListingBody {
    pub fn into_insert_listing(self, author: Uuid) -> InsertListing {
        InsertListing { title: self.title,
            description: self.description,
            author,
            listing_type: self.listing_type,
            tradeable: Some(self.tradeable),
            thumbnail: self.thumbnail
        }
    }
}

async fn create_listing(
    auth_session: AuthSession,
    State(backend): State<Backend>,
    Json(body): Json<InsertListingBody>
) -> impl IntoResponse {
    if body.pictures.is_empty() {
        return (StatusCode::BAD_REQUEST, "Pictures are required").into_response();
    }

    if !body.pictures.contains(&body.thumbnail) {
        return (StatusCode::BAD_REQUEST, "Thumbnail is not in the pictures supplied").into_response();
    }

    let author_id = auth_session.user.as_ref().unwrap().claims.user_id;

    let insert_listing = body.into_insert_listing(author_id);

    match backend.create_listing(insert_listing).await {
        Ok(listing) => {
            (StatusCode::CREATED, Json(listing)).into_response()
        }
        Err(err) => {
            error!(?err, "Database error while creating listing");
            (StatusCode::INTERNAL_SERVER_ERROR, "Error while creating listing").into_response()
        }
    }
}

async fn get_all_listings(
    _auth_session: AuthSession,
    State(backend): State<Backend>
) -> impl IntoResponse {
    match backend.get_all_listings().await {
        Ok(listings) => {
            Json(listings).into_response()
        }
        Err(err) => {
            error!(?err, "Error while getting all listings");
            (StatusCode::INTERNAL_SERVER_ERROR, "Error while getting all listings")
                .into_response()
        }
    }
}

async fn get_listing(
    _auth_session: AuthSession,
    State(backend): State<Backend>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match backend.get_listing(id).await {
        Ok(Some(listing)) => {
            (StatusCode::OK, Json(listing))
                .into_response()
        }
        Ok(None) => {
            StatusCode::NO_CONTENT.into_response()
        }
        Err(err) => {
            error!(?err, ?id, "Error while getting listing");
            (StatusCode::INTERNAL_SERVER_ERROR, "Error while getting listing")
                .into_response()
        }
    }
}

async fn update_listing(
    _auth_session: AuthSession,
    State(backend): State<Backend>,
    Path(id): Path<Uuid>,
    Json(mut listing_update): Json<ListingUpdate>,
) -> impl IntoResponse {
    listing_update.id = Some(id);

    match backend.update_listing(&listing_update).await {
        Ok(Some(listing)) => {
            (StatusCode::ACCEPTED, Json(listing)).into_response()
        }
        Ok(None) => {
            (StatusCode::BAD_REQUEST, "Invalid ID").into_response()
        }
        Err(err) => {
            error!(?err, ?listing_update, "Database error while trying to update listing");
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}
