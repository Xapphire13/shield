use std::collections::HashMap;

use anyhow::anyhow;
use axum::{Json, extract::State, http::StatusCode};
use futures::future::join_all;
use shield_models::{RecordingMode, SetRecordingModeRequest};
use tracing::info;
use unifi_protect_client::models::camera::{CameraUpdateBuilder, RecordingSettingsUpdateBuilder};

use crate::{AppState, app_error::AppError, handlers::AuthenticatedRequestContext};

pub async fn set_recording_mode(
    State(AppState {
        client,
        notification_dispatcher,
        config,
        ..
    }): State<AppState>,
    _: AuthenticatedRequestContext,
    Json(input): Json<SetRecordingModeRequest>,
) -> Result<StatusCode, AppError> {
    info!(
        "Setting recording mode to {:?} for cameras: {:?}",
        input.mode, input.camera_ids
    );

    {
        let client = client.clone();
        let input = input.clone();
        tokio::spawn(async move {
            if let Some(notifications_config) = config.notifications.as_ref() {
                let camera_names_by_id: HashMap<String, String> = client
                    .list_cameras()
                    .await
                    .unwrap_or(Vec::new())
                    .into_iter()
                    .map(|camera| (camera.id, camera.name))
                    .collect();
                let payload = ntfy::Payload::new(notifications_config.topic.clone())
                    .title(match input.mode {
                        RecordingMode::Always => "Recording turned on",
                        RecordingMode::Schedule => "Recording schedule changed",
                        RecordingMode::Never => "Recording turned off",
                    })
                    .message(format!(
                        "For cameras:\n\n{}",
                        input
                            .camera_ids
                            .iter()
                            .map(|id| camera_names_by_id.get(id).unwrap_or(id).clone())
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                let _ = notification_dispatcher.send(&payload).await.ok();
            }
        });
    }

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
