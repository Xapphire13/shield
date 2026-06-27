use axum::{
    Router,
    http::{StatusCode, Uri},
    middleware,
    routing::{delete, get, patch, post},
};
use tracing::error;

use crate::{
    AppState, handlers,
    middleware::auth_backoff::{AuthBackoffState, auth_backoff_middleware},
};

pub fn create_routes() -> Router<AppState> {
    // Auth routes with exponential backoff
    let auth_backoff_state = AuthBackoffState::new(900); // 15 minute reset window
    let auth_router = Router::new()
        .route("/authenticate", post(handlers::authenticate))
        .route("/refresh", post(handlers::refresh))
        .layer(middleware::from_fn_with_state(
            auth_backoff_state.clone(),
            auth_backoff_middleware,
        ));

    // Other routes without backoff
    let other_routes = Router::new()
        .route("/cameras", get(handlers::list_cameras))
        .route("/set_recording_mode", post(handlers::set_recording_mode))
        .route("/maps/{id}", get(handlers::get_map))
        .route("/maps/{id}/cameras", post(handlers::add_map_camera))
        .route(
            "/maps/{id}/cameras/{camera_id}",
            patch(handlers::update_map_camera),
        )
        .route(
            "/maps/{id}/cameras/{camera_id}",
            delete(handlers::delete_map_camera),
        );

    Router::new()
        .merge(auth_router)
        .merge(other_routes)
        .fallback(fallback)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    error!("No route for {uri}");
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}
