use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    Json, Router, debug_handler,
    extract::State,
    http::{Request, Response, StatusCode, Uri, header::CONTENT_TYPE},
    routing::{get, post},
};
use futures::future::join_all;
use shield_models::{Camera, RecordingMode, RecordingSettings, SetRecordingModeInput};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{Level, error, info, trace};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};
use unifi_protect_client::{
    UnifiProtectClient,
    models::camera::{CameraUpdateBuilder, RecordingSettingsUpdateBuilder},
};

use crate::{app_error::AppError, config::Config};

mod app_error;
mod config;

#[tokio::main]
async fn main() {
    let filter = filter::Targets::new().with_target("shield_service", Level::TRACE);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    let config = Config::load();
    let client_state = Arc::new(UnifiProtectClient::new(
        "https://192.168.1.1",
        &config.credentials.username,
        &config.credentials.password,
    ));
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers([CONTENT_TYPE]);
    let app = Router::new()
        .route("/cameras", get(list_cameras))
        .route("/set_recording_mode", post(set_recording_mode))
        .fallback(fallback)
        .with_state(client_state)
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_request(|_request: &Request<_>, _span: &tracing::Span| {
                    trace!("request started");
                })
                .on_response(
                    |response: &Response<_>,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        trace!(
                            status = response.status().as_u16(),
                            latency_ms = latency.as_millis(),
                            "request completed"
                        );
                    },
                ),
        );

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

async fn set_recording_mode(
    State(client): State<Arc<UnifiProtectClient>>,
    Json(payload): Json<SetRecordingModeInput>,
) -> Result<StatusCode, AppError> {
    info!(
        "Setting recording mode to {:?} for cameras: {:?}",
        payload.mode, payload.camera_ids
    );

    let futures = payload.camera_ids.iter().map(|camera_id| {
        client.update_camera(
            camera_id,
            CameraUpdateBuilder::new()
                .with_recording_settings(
                    RecordingSettingsUpdateBuilder::new()
                        .with_mode(match payload.mode {
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
                payload.camera_ids.get(i).unwrap()
            ))?;
        }
    }

    Ok(StatusCode::OK)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    error!("No route for {uri}");
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}
