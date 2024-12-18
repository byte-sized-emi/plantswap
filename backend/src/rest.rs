use axum::Router;

use crate::AppState;

mod listings;
mod pictures;
mod plants;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/listing", listings::router())
        .nest("/picture", pictures::router())
        .nest("/plant", plants::router())
}
