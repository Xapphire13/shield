use axum::{
    Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use shield_models::UpdateMapWallRequest;
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn update_map_wall(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path((id, wall_id)): Path<(String, String)>,
    Json(update): Json<UpdateMapWallRequest>,
) -> Result<Response, AppError> {
    info!("Updating wall {wall_id} on map {id}");

    match map_store.update_wall(&id, &wall_id, update)? {
        Some(wall) => Ok(Json(wall).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
