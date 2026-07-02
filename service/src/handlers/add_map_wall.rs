use axum::{
    Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
};
use shield_models::MapWall;
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn add_map_wall(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path(id): Path<String>,
    Json(wall): Json<MapWall>,
) -> Result<(StatusCode, Json<MapWall>), AppError> {
    info!("Adding wall {} to map {id}", wall.id);

    map_store.add_wall(&id, wall.clone())?;

    Ok((StatusCode::CREATED, Json(wall)))
}
