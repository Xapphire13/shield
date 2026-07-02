use axum::{
    Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use shield_models::UpdateMapDoorRequest;
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn update_map_door(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path((id, door_id)): Path<(String, String)>,
    Json(update): Json<UpdateMapDoorRequest>,
) -> Result<Response, AppError> {
    info!("Updating door {door_id} on map {id}");

    match map_store.update_door(&id, &door_id, update)? {
        Some(door) => Ok(Json(door).into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
