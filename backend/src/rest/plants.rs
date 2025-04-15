use axum::{extract::State, routing::post, Json, http::StatusCode, Router, response::IntoResponse};
use futures::future::try_join_all;
use postgis_diesel::types::Point;
use serde::Deserialize;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{auth::AuthSession, backend::{recognition::{PlantRecogniser, PlantRecognitionInfo}, Backend}, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/recognise", post(recognise_plant))
}

#[derive(Deserialize, Debug, Clone)]
struct RecognisePlantInput {
    pub images: Vec<Uuid>,
    #[serde(default)]
    pub location: Option<Location>,
}

#[derive(Deserialize, Debug, Clone)]
struct Location {
    x: f64,
    y: f64
}

impl Location {
    /// Rounds this location to 1 decimal place (approx. 11.1km).
    /// Source: https://support.garmin.com/en-US/?faq=hRMBoCTy5a7HqVkxukhHd8
    pub fn round(&self) -> Location {
        Location {
            x: (self.x * 10.0).round() / 10.0,
            y: (self.y * 10.0).round() / 10.0,
        }
    }

    pub fn to_point(&self) -> Point {
        let rounded = self.round();
        Point::new(rounded.x, rounded.y, None)
    }
}

async fn recognise_plant(
    _auth_session: AuthSession,
    State(backend): State<Backend>,
    Json(input): Json<RecognisePlantInput>,
) -> impl IntoResponse {
    let image_uuids = input.images;

    let fetch_image_results: Result<Vec<_>, _> = try_join_all(image_uuids.iter()
        .map(|uuid| async {
            backend.get_image(*uuid).await
        }))
        .await;

    let maybe_missing_images = match fetch_image_results {
        Ok(images) => images,
        Err(err) => {
            error!(?err, "Error while trying to download image");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error while trying to download image")
                .into_response();
        }
    };

    if let Some(index) = maybe_missing_images.iter().position(|image| image.is_none()) {
        let uuid = image_uuids[index];
        warn!(?uuid, "Couldn't find image");
        return (StatusCode::BAD_REQUEST, format!("Couldn't find image with uuid={uuid}"))
            .into_response();
    }

    let images = maybe_missing_images.into_iter()
        .map(|img| img.unwrap().1)
        .zip(image_uuids.iter().map(Uuid::to_string))
        .collect();

    let location = input.location.map(|l| l.to_point());

    let info = PlantRecognitionInfo {
        images,
        location,
    };

    let mut db = backend.db.lock().await;

    let plant_analysis = backend.plant_recognition.analyze_plant(&mut db, &info).await;

    match plant_analysis {
        Ok(plants) => (StatusCode::OK, Json(plants)).into_response(),
        Err(err) => {
            error!(?err, "Plant analysis failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "Plant analysis failed")
                .into_response()
        }
    }
}
