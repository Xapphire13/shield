use axum::{
    Router,
    http::{StatusCode, Uri},
    middleware,
    routing::{get, patch, post},
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
        .route("/set_recording_mode", post(handlers::set_recording_mode));

    // Map CRUD routes, nested under /maps
    let maps_router = Router::new()
        .route("/{id}", get(handlers::get_map))
        .route("/{id}/cameras", post(handlers::add_map_camera))
        .route(
            "/{id}/cameras/{camera_id}",
            patch(handlers::update_map_camera).delete(handlers::delete_map_camera),
        )
        .route("/{id}/walls", post(handlers::add_map_wall))
        .route(
            "/{id}/walls/{wall_id}",
            patch(handlers::update_map_wall).delete(handlers::delete_map_wall),
        )
        .route("/{id}/doors", post(handlers::add_map_door))
        .route(
            "/{id}/doors/{door_id}",
            patch(handlers::update_map_door).delete(handlers::delete_map_door),
        );

    Router::new()
        .merge(auth_router)
        .merge(other_routes)
        .nest("/maps", maps_router)
        .fallback(fallback)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    error!("No route for {uri}");
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}
