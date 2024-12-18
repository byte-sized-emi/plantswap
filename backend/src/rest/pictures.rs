use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::{extract::{Path, State}, Router};
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use serde::Serialize;
use tracing::error;
use uuid::Uuid;

use crate::{auth::AuthSession, backend::Backend, AppState};


pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(upload_picture))
        .route("/:id", get(get_picture).delete(remove_picture))
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
            return (
                StatusCode::CREATED,
                Json(PictureUploadResponse { id })
            ).into_response()
        }
        Err(err) => {
            error!(?err, "Couldn't upload image");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Couldn't upload image"
            ).into_response()
        }
    }
}
