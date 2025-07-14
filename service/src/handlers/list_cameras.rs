use axum::{
    Json, debug_handler,
    extract::State,
};
use shield_models::{
    Camera, RecordingMode, RecordingSettings,
};
use tracing::info;

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

#[debug_handler]
pub async fn list_cameras(
    State(AppState { client, .. }): State<AppState>,
    _: AuthenticatedRequestContext,
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
