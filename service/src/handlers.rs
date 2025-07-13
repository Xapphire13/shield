use anyhow::anyhow;
use axum::{
    Json, debug_handler,
    extract::State,
    http::StatusCode,
};
use chrono::{Days, Utc};
use futures::future::join_all;
use jsonwebtoken::{EncodingKey, Header};
use serde::Serialize;
use shield_models::{
    AuthenticateRequest, AuthenticationResponse, Camera, RecordingMode, RecordingSettings,
    SetRecordingModeRequest,
};
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::info;
use unifi_protect_client::models::camera::{CameraUpdateBuilder, RecordingSettingsUpdateBuilder};

use crate::{AppState, app_error::AppError};

#[debug_handler]
pub async fn list_cameras(
    State(AppState { client, .. }): State<AppState>,
) -> Result<Json<Vec<shield_models::Camera>>, AppError> {
    info!("Fetching cameras");

    let tags = client.get_device_tags().await?;
    let cameras: Vec<Camera> = client
        .list_cameras()
        .await?
        .into_iter()
        .map(|camera| {
            let tags = tags
                .iter()
                .filter_map(|tag| {
                    if tag.device_macs.contains(&camera.mac) {
                        Some(tag.name.clone())
                    } else {
                        None
                    }
                })
                .collect();

            Camera {
                id: camera.id,
                name: camera.name,
                recording_settings: RecordingSettings {
                    mode: match camera.recording_settings.mode {
                        unifi_protect_client::models::camera::RecordingMode::Always => {
                            RecordingMode::Always
                        }
                        unifi_protect_client::models::camera::RecordingMode::Schedule => {
                            RecordingMode::Schedule
                        }
                        unifi_protect_client::models::camera::RecordingMode::Never => {
                            RecordingMode::Never
                        }
                    },
                },
                is_recording: camera.is_recording,
                tags,
            }
        })
        .collect();

    Ok(Json(cameras))
}

pub async fn set_recording_mode(
    State(AppState { client, .. }): State<AppState>,
    Json(input): Json<SetRecordingModeRequest>,
) -> Result<StatusCode, AppError> {
    info!(
        "Setting recording mode to {:?} for cameras: {:?}",
        input.mode, input.camera_ids
    );

    let futures = input.camera_ids.iter().map(|camera_id| {
        client.update_camera(
            camera_id,
            CameraUpdateBuilder::new()
                .with_recording_settings(
                    RecordingSettingsUpdateBuilder::new()
                        .with_mode(match input.mode {
                            RecordingMode::Always => {
                                unifi_protect_client::models::camera::RecordingMode::Always
                            }
                            RecordingMode::Schedule => {
                                unifi_protect_client::models::camera::RecordingMode::Schedule
                            }
                            RecordingMode::Never => {
                                unifi_protect_client::models::camera::RecordingMode::Never
                            }
                        })
                        .build(),
                )
                .build(),
        )
    });

    let results = join_all(futures).await;

    for (i, result) in results.iter().enumerate() {
        if result.is_err() {
            Err(anyhow!(
                "Issue updating camera {}",
                input.camera_ids.get(i).unwrap()
            ))?;
        }
    }

    Ok(StatusCode::OK)
}

#[derive(Serialize)]
struct JwtClaims {
    exp: usize,
}

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
        Err(anyhow!("Invalid OTP code"))?;
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
