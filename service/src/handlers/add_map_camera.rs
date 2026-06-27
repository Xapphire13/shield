use axum::{
    Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
};
use shield_models::MapCamera;
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn add_map_camera(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path(id): Path<String>,
    Json(camera): Json<MapCamera>,
) -> Result<(StatusCode, Json<MapCamera>), AppError> {
    info!("Adding camera {} to map {id}", camera.camera_id);

    map_store.add_camera(&id, camera.clone())?;

    Ok((StatusCode::CREATED, Json(camera)))
}
