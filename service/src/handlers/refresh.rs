use axum::{Json, debug_handler, extract::State};
use shield_models::{AuthenticationResponse, RefreshRequest};
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::create_auth_token};

#[debug_handler]
pub async fn refresh(
    State(AppState {
        refresh_token_store,
        config,
        ..
    }): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<AuthenticationResponse>, AppError> {
    refresh_token_store.validate_token(request.refresh_token)?;

    let token = create_auth_token(&config)?;
    let refresh_token = refresh_token_store.generate_new_token()?;

    info!("Refresh token valid, generating new auth token");

    Ok(Json(AuthenticationResponse {
        token,
        refresh_token: refresh_token.token,
    }))
}
