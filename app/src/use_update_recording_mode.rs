use dioxus::prelude::*;
use shield_models::{RecordingMode, SetRecordingModeRequest};

use crate::get_api_url;

pub fn use_update_recording_mode() -> impl Fn(Vec<String>, RecordingMode) {
    |ids, mode| {
        let url = get_api_url("/set_recording_mode");

        spawn(async move {
            let res = reqwest::Client::new()
                .post(url)
                .json(&SetRecordingModeRequest {
                    camera_ids: ids,
                    mode,
                })
                .send()
                .await;

            match res {
                Ok(res) if res.status().is_success() => {
                    web_sys::window().unwrap().location().reload().unwrap()
                }
                _ => {}
            }
        });
    }
}
