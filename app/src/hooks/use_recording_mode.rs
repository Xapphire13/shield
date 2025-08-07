use dioxus::prelude::*;
use shield_models::RecordingMode;

use crate::{api::CameraApi, hooks::use_api_client::use_api_client, utils::reload_page};

pub fn use_update_recording_mode() -> impl Fn(Vec<String>, RecordingMode) {
    let client = use_api_client();

    move |ids, mode| {
        let client = client.clone();
        spawn(async move {
            let result = client.update_recording_mode(ids, mode).await;

            if result.is_ok() {
                let _ = reload_page();
            }
        });
    }
}
