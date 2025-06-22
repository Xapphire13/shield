use dioxus::prelude::*;
use shield_models::RecordingMode;

use crate::components::{RecordingIndicator, ui::DisclosureRow};

#[component]
pub fn Camera(camera: shield_models::Camera) -> Element {
    let recording_mode = match camera.recording_settings.mode {
        RecordingMode::Always => "Always",
        RecordingMode::Schedule => "Schedule",
        RecordingMode::Never => "Never",
    };

    rsx! {
        DisclosureRow {
            header: camera.name,
            sub_header: "Recording mode: {recording_mode}",
            after: rsx! {
                RecordingIndicator { is_recording: camera.is_recording }
            },
        }
    }
}
