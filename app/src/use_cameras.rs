use dioxus::prelude::*;

use crate::use_api_client::use_api_client;

pub struct UseCamerasResult {
    pub cameras: Vec<shield_models::Camera>,
    pub loading: bool,
}

pub fn use_cameras() -> UseCamerasResult {
    let client = use_api_client();
    let cameras = use_resource(move || {
        let client = client.clone();
        async move { client.get_cameras().await.unwrap_or(Vec::new()) }
    });

    UseCamerasResult {
        cameras: cameras().unwrap_or_default(),
        loading: !cameras.finished(),
    }
}
