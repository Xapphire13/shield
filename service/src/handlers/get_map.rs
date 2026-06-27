use axum::{
    Json, debug_handler,
    extract::{Path, State},
};
use shield_models::Map;
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn get_map(
    State(AppState { map_store, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
    Path(id): Path<String>,
) -> Result<Json<Map>, AppError> {
    info!("Fetching map {id}");

    Ok(Json(map_store.get_map(&id)?))
}
