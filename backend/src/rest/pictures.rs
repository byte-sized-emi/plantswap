use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::Json;
use axum::{extract::State, Router};
use axum_login::login_required;
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use serde::Serialize;
use tracing::{error, warn};
use uuid::Uuid;

use crate::auth::AuthState;
use crate::{auth::AuthSession, backend::Backend, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(upload_picture))
        .route("/:id", delete(remove_picture))
        .route_layer(login_required!(AuthState, login_url = crate::LOGIN_URL))
        .route("/:id", get(get_picture))

}

#[derive(TryFromMultipart)]
struct PictureUpload {
    #[form_data(limit = "10MiB")]
    pub picture: FieldData<axum::body::Bytes>,
}

#[derive(Serialize)]
struct PictureUploadResponse {
    pub id: Uuid
}

async fn upload_picture(
    auth_session: AuthSession,
    State(backend): State<Backend>,
    TypedMultipart(picture_upload): TypedMultipart<PictureUpload>
) -> impl IntoResponse {
    let picture = picture_upload.picture;

    let content_type = picture.metadata.content_type
        .as_ref()
        .map(|f| f.as_ref());
    if content_type != Some("image/jpeg") && content_type != Some("image/png") {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!("Media type \"{content_type:?}\" is not supported, use \"image/png\" or \"image/jpeg\""))
                .into_response();
    }

    let user_id = auth_session.user.unwrap().claims.user_id;

    match backend.upload_image(user_id, picture.contents).await {
        Ok(id) => {
            (
                StatusCode::CREATED,
                Json(PictureUploadResponse { id })
            ).into_response()
        }
        Err(err) => {
            error!(?err, "Couldn't upload image");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Couldn't upload image"
            ).into_response()
        }
    }
}

async fn get_picture(
    State(backend): State<Backend>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match backend.get_image(id).await {
        Err(err) =>{
            warn!(?err, image_id = ?id, "Error while trying to download image");
            (StatusCode::INTERNAL_SERVER_ERROR, "Error occured trying to download image from S3")
            .into_response()
        },
        Ok(Some(bytes)) =>
            ([
                (header::CACHE_CONTROL, "public, max-age=604800, immutable".to_string()),
                (header::CONTENT_TYPE, bytes.0)
                ], bytes.1)
                .into_response(),
        Ok(None) =>
            (StatusCode::NOT_FOUND, "Couldn't find this image")
                .into_response()
    }
}

async fn remove_picture() {
    todo!()
}
