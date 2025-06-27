use std::sync::Arc;

use axum::{Json, Router, debug_handler, extract::State, routing::get};
use log::info;
use shield_models::{Camera, RecordingMode, RecordingSettings};
use tower_http::cors::{Any, CorsLayer};
use unifi_protect_client::UnifiProtectClient;

use crate::{app_error::AppError, credentials::Credentials};

mod app_error;
mod credentials;

#[tokio::main]
async fn main() {
    env_logger::init();
    let credentials = Credentials::load();
    let client_state = Arc::new(UnifiProtectClient::new(
        "https://192.168.1.1",
        &credentials.username,
        &credentials.password,
    ));
    let cors = CorsLayer::new().allow_origin(Any);
    let app = Router::new()
        .route("/cameras", get(list_cameras))
        .with_state(client_state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Listening on port 3000");
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
async fn list_cameras(
    State(client): State<Arc<UnifiProtectClient>>,
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
