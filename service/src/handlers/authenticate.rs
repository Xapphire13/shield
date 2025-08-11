use anyhow::anyhow;
use axum::{Json, debug_handler, extract::State};
use chrono::{TimeDelta, Utc};
use jsonwebtoken::{EncodingKey, Header};
use shield_models::{AuthToken, AuthenticateRequest, AuthenticationResponse};
use totp_rs::{Algorithm, Secret, TOTP};

use crate::{AppState, app_error::AppError, config::Config, handlers::JwtClaims};

#[debug_handler]
pub async fn authenticate(
    State(AppState {
        config,
        refresh_token_store,
        notification_dispatcher,
        ..
    }): State<AppState>,
    Json(request): Json<AuthenticateRequest>,
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

    if !totp.check_current(&request.otp_code)? {
        tokio::spawn(async move {
            if let Some(notifications_config) = config.notifications.as_ref() {
                let payload = ntfy::Payload::new(notifications_config.topic.clone())
                    .title("New login")
                    .message("Failed");
                let _ = notification_dispatcher.send(&payload).await.ok();
            }
        });

        return Err(AppError::InvalidOtpCode);
    }

    let token = create_auth_token(&config)?;
    let refresh_token = refresh_token_store.generate_new_token()?;

    tokio::spawn(async move {
        if let Some(notifications_config) = config.notifications.as_ref() {
            let payload = ntfy::Payload::new(notifications_config.topic.clone())
                .title("New login")
                .message("Successful");

            let _ = notification_dispatcher.send(&payload).await.ok();
        }
    });

    Ok(Json(AuthenticationResponse {
        token,
        refresh_token: refresh_token.token,
    }))
}

/// Auth tokens expire after 30 minutes
pub fn create_auth_token(config: &Config) -> anyhow::Result<AuthToken> {
    let exp = Utc::now()
        .checked_add_signed(TimeDelta::minutes(30))
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

    Ok(token)
}
