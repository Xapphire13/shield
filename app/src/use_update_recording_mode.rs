use dioxus::prelude::*;
use shield_models::{RecordingMode, SetRecordingModeRequest};

use crate::use_api_client::use_api_client;

pub fn use_update_recording_mode() -> impl Fn(Vec<String>, RecordingMode) {
    let client = use_api_client();

    move |ids, mode| {
        let client = client.clone();
        spawn(async move {
            let result = client
                .set_recording_mode(SetRecordingModeRequest {
                    camera_ids: ids,
                    mode,
                })
                .await;

            if result.is_ok() {
                web_sys::window().unwrap().location().reload().unwrap()
            }
        });
    }
}
