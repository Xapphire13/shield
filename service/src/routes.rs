use axum::{
    Router,
    http::{StatusCode, Uri},
    routing::{get, post},
};
use tracing::error;

use crate::{AppState, handlers};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/cameras", get(handlers::list_cameras))
        .route("/set_recording_mode", post(handlers::set_recording_mode))
        .route("/authenticate", post(handlers::authenticate))
        .fallback(fallback)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    error!("No route for {uri}");
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}
