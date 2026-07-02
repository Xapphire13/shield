mod add_map_camera;
mod add_map_door;
mod add_map_wall;
mod authenticate;
mod delete_map_camera;
mod delete_map_door;
mod delete_map_wall;
mod get_map;
mod list_cameras;
mod refresh;
mod set_recording_mode;
mod update_map_camera;
mod update_map_door;
mod update_map_wall;

pub use add_map_camera::*;
pub use add_map_door::*;
pub use add_map_wall::*;
pub use authenticate::*;
pub use delete_map_camera::*;
pub use delete_map_door::*;
pub use delete_map_wall::*;
pub use get_map::*;
pub use list_cameras::*;
pub use refresh::*;
pub use set_recording_mode::*;
pub use update_map_camera::*;
pub use update_map_door::*;
pub use update_map_wall::*;

use anyhow::anyhow;
use axum::{RequestPartsExt, extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

use crate::{AppState, app_error::AppError};

pub struct AuthenticatedRequestContext;

impl FromRequestParts<AppState> for AuthenticatedRequestContext {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        AppState { config, .. }: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::InvalidToken)?;

        match decode::<JwtClaims>(
            bearer.token(),
            &DecodingKey::from_secret(
                config
                    .jwt
                    .as_ref()
                    .ok_or(AppError::Unknown(anyhow!("Can't load jwt secret")))?
                    .secret
                    .as_ref(),
            ),
            &Validation::default(),
        ) {
            Ok(_) => Ok(AuthenticatedRequestContext),
            Err(err) => Err(match err.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::ExpiredToken,
                _ => AppError::InvalidToken,
            }),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct JwtClaims {
    exp: usize,
}
