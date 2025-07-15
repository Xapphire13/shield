use anyhow::anyhow;
use axum::{Json, debug_handler, extract::State};
use chrono::{Days, Utc};
use jsonwebtoken::{EncodingKey, Header};
use shield_models::{AuthenticateRequest, AuthenticationResponse};
use totp_rs::{Algorithm, Secret, TOTP};

use crate::{AppState, app_error::AppError, handlers::JwtClaims};

#[debug_handler]
pub async fn authenticate(
    State(AppState { config, .. }): State<AppState>,
    Json(input): Json<AuthenticateRequest>,
) -> Result<Json<AuthenticationResponse>, AppError> {
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(
            config
                .otp
                .as_ref()
                .ok_or(anyhow!("Couldn't get OTP secret"))?
                .secret
                .clone(),
        )
        .to_bytes()?,
    )?;

    if !totp.check_current(&input.otp_code)? {
        return Err(AppError::InvalidOtpCode);
    }

    let exp = Utc::now()
        .checked_add_days(Days::new(30))
        .ok_or(anyhow!("Couldn't calculate JWT expiry"))?;

    let token = jsonwebtoken::encode(
        &Header::default(),
        &JwtClaims {
            exp: exp.timestamp() as usize,
        },
        &EncodingKey::from_secret(
            config
                .jwt
                .as_ref()
                .ok_or(anyhow!("Couldn't get JWT secret"))?
                .secret
                .as_ref(),
        ),
    )?;

    Ok(Json(AuthenticationResponse { token }))
}
