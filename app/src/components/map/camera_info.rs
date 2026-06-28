use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::fi_icons::FiX;
use shield_models::{Camera, RecordingMode};

use crate::components::ui::{ButtonColor, IconButton};

/// Compact, read-only info card for a camera tapped on the map in view mode.
///
/// Mirrors the edit-mode [`CameraInspector`](super::camera_inspector) styling for
/// a consistent look, but exposes no edit controls. It floats above the global
/// List/Map navigation toolbar (view mode keeps the nav reachable) and offers a
/// close affordance; the host also clears it when leaving edit mode or when the
/// underlying canvas is tapped.
#[component]
pub fn CameraInfo(
    /// The resolved camera for the tapped marker, or `None` when its placed
    /// reference is an orphan (the underlying camera was deleted).
    camera: Option<Camera>,
    /// Dismiss the card.
    on_close: Callback,
) -> Element {
    let orphaned = camera.is_none();
    let title = camera
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Unknown camera".to_string());

    rsx! {
        div { class: "camera-info",
            div { class: "camera-info__header",
                div {
                    class: "camera-info__title",
                    "data-orphaned": orphaned,
                    "{title}"
                }
                IconButton {
                    icon: rsx! {
                        Icon { width: 20, height: 20, icon: FiX }
                    },
                    color: ButtonColor::Default,
                    on_press: move |_| on_close(()),
                }
            }

            if let Some(camera) = camera {
                div { class: "camera-info__rows",
                    div { class: "camera-info__row",
                        span { class: "camera-info__label", "Status" }
                        span {
                            class: "camera-info__value",
                            "data-recording": camera.is_recording,
                            if camera.is_recording {
                                "Recording"
                            } else {
                                "Not recording"
                            }
                        }
                    }
                    div { class: "camera-info__row",
                        span { class: "camera-info__label", "Recording mode" }
                        span { class: "camera-info__value",
                            "{recording_mode_label(&camera.recording_settings.mode)}"
                        }
                    }
                }

                if !camera.tags.is_empty() {
                    div { class: "camera-info__tags",
                        for tag in camera.tags.iter() {
                            span { key: "{tag}", class: "camera-info__tag", "{tag}" }
                        }
                    }
                }
            } else {
                div { class: "camera-info__note",
                    "This camera no longer exists."
                }
            }
        }
    }
}

/// Human-readable label for a recording mode.
fn recording_mode_label(mode: &RecordingMode) -> &'static str {
    match mode {
        RecordingMode::Always => "Always",
        RecordingMode::Schedule => "Schedule",
        RecordingMode::Never => "Never",
    }
}
