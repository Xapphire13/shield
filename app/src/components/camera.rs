pub mod recording_indicator;

pub use recording_indicator::RecordingIndicator;

use dioxus::prelude::*;

use crate::{components::ui::SelectableRow, utils::RecordingModeExtensions};

#[component]
pub fn Camera(
    camera: shield_models::Camera,
    selected: bool,
    on_select: Callback<String>,
) -> Element {
    let recording_mode = camera.recording_settings.mode.display_name();
    let id = camera.id.clone();

    rsx! {
        SelectableRow {
            header: camera.name,
            sub_header: "Recording mode: {recording_mode}",
            selected,
            on_click: move |_| on_select.call(id.clone()),
            after: rsx! {
                RecordingIndicator { is_recording: camera.is_recording }
            },
        }
    }
}
