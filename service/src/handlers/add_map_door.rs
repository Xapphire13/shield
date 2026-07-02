use axum::{
    Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
};
use shield_models::MapDoor;
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn add_map_door(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path(id): Path<String>,
    Json(door): Json<MapDoor>,
) -> Result<(StatusCode, Json<MapDoor>), AppError> {
    info!("Adding door {} to map {id}", door.id);

    map_store.add_door(&id, door.clone())?;

    Ok((StatusCode::CREATED, Json(door)))
}
