use std::sync::Arc;

use axum::{Json, Router, debug_handler, extract::State, routing::get};
use unifi_protect_client::{UnifiProtectClient, models::camera::Camera};

use crate::{app_error::AppError, credentials::Credentials};

mod app_error;
mod credentials;

#[tokio::main]
async fn main() {
    let credentials = Credentials::load();
    let client_state = Arc::new(UnifiProtectClient::new(
        "https://192.168.1.1",
        &credentials.username,
        &credentials.password,
    ));
    let app = Router::new()
        .route("/cameras", get(list_cameras))
        .with_state(client_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[debug_handler]
async fn list_cameras(
    State(client): State<Arc<UnifiProtectClient>>,
) -> Result<Json<Vec<Camera>>, AppError> {
    let cameras = client.list_cameras().await?;

    Ok(Json(cameras))
}
