use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn delete_map_camera(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path((id, camera_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    info!("Removing camera {camera_id} from map {id}");

    match map_store.remove_camera(&id, &camera_id)? {
        Some(()) => Ok(StatusCode::NO_CONTENT),
        None => Ok(StatusCode::NOT_FOUND),
    }
}
