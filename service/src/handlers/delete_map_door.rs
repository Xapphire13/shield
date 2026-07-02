use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn delete_map_door(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path((id, door_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    info!("Removing door {door_id} from map {id}");

    match map_store.remove_door(&id, &door_id)? {
        Some(()) => Ok(StatusCode::NO_CONTENT),
        None => Ok(StatusCode::NOT_FOUND),
    }
}
