mod authenticate;
mod list_cameras;
mod set_recording_mode;

pub use authenticate::*;
pub use list_cameras::*;
pub use set_recording_mode::*;

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
