pub mod recording_indicator;

pub use recording_indicator::RecordingIndicator;

use dioxus::prelude::*;

use crate::{components::ui::DisclosureRow, utils::RecordingModeExtensions};

#[component]
pub fn Camera(camera: shield_models::Camera) -> Element {
    let recording_mode = camera.recording_settings.mode.display_name();

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
