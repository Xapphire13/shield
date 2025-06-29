use dioxus::prelude::*;
use shield_models::{RecordingMode, SetRecordingModeInput};

use crate::get_api_url;

pub fn use_update_recording_mode() -> impl Fn(Vec<String>, RecordingMode) {
    let update_recording_mode = |ids, mode| {
        let url = get_api_url("/set_recording_mode");

        spawn(async move {
            let res = reqwest::Client::new()
                .post(url)
                .json(&SetRecordingModeInput {
                    camera_ids: ids,
                    mode,
                })
                .send()
                .await
                .unwrap();

            if res.status().is_success() {
                web_sys::window().unwrap().location().reload().unwrap();
            }
        });
    };

    update_recording_mode
}
