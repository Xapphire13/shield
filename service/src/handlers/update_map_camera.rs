use axum::{
    Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use shield_models::UpdateMapCameraRequest;
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn update_map_camera(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path((id, camera_id)): Path<(String, String)>,
    Json(update): Json<UpdateMapCameraRequest>,
) -> Result<Response, AppError> {
    info!("Updating camera {camera_id} on map {id}");

    match map_store.update_camera(&id, &camera_id, update)? {
        Some(camera) => Ok(Json(camera).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
