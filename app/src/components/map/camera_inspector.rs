use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::fi_icons::FiTrash2;
use shield_models::FieldOfView;

use crate::components::ui::{ButtonColor, IconButton};

/// Contextual bottom sheet for the currently selected map camera. Exposes
/// sliders for the field-of-view (direction / field-of-view angle / range) and a
/// delete action.
///
/// Sliders preview live while dragging and commit once on release: every
/// intermediate value fires `on_preview_fov` (canvas-only, no persistence or
/// undo entry), while the final value on release fires `on_change_fov` (the
/// single committed edit). This keeps the cone in sync with the thumb in real
/// time while keeping the undo stack to one entry per interaction.
#[component]
pub fn CameraInspector(
    /// Display name of the camera, or `None` when the placed reference is an
    /// orphan (the underlying camera was deleted).
    name: Option<String>,
    /// Current field-of-view for the selected camera. When a preview is active
    /// this already reflects the in-progress value, keeping thumb and cone synced.
    fov: FieldOfView,
    /// Preview a field-of-view without committing (fired on every slider move).
    on_preview_fov: Callback<FieldOfView>,
    /// Commit a new field-of-view (fired once on slider release).
    on_change_fov: Callback<FieldOfView>,
    /// Remove this camera from the map.
    on_delete: Callback,
) -> Element {
    let title = name.clone().unwrap_or_else(|| "Unknown camera".to_string());
    let orphaned = name.is_none();

    rsx! {
        div { class: "camera-inspector",
            div { class: "camera-inspector__header",
                div {
                    class: "camera-inspector__title",
                    "data-orphaned": orphaned,
                    "{title}"
                }
                IconButton {
                    icon: rsx! {
                        Icon { width: 20, height: 20, icon: FiTrash2 }
                    },
                    color: ButtonColor::Danger,
                    on_press: move |_| on_delete(()),
                }
            }

            if orphaned {
                div { class: "camera-inspector__note",
                    "This camera no longer exists. Remove it from the map."
                }
            }

            FovSlider {
                label: "Direction",
                unit: "°",
                min: 0,
                max: 359,
                value: fov.direction_deg as i32,
                on_preview: {
                    let fov = fov.clone();
                    move |value: i32| {
                        on_preview_fov(FieldOfView {
                            direction_deg: value as u16,
                            ..fov.clone()
                        });
                    }
                },
                on_change: {
                    let fov = fov.clone();
                    move |value: i32| {
                        on_change_fov(FieldOfView {
                            direction_deg: value as u16,
                            ..fov.clone()
                        });
                    }
                },
            }

            FovSlider {
                label: "Field of View",
                unit: "°",
                min: 1,
                max: 359,
                value: fov.angle_deg as i32,
                on_preview: {
                    let fov = fov.clone();
                    move |value: i32| {
                        on_preview_fov(FieldOfView {
                            angle_deg: value as u16,
                            ..fov.clone()
                        });
                    }
                },
                on_change: {
                    let fov = fov.clone();
                    move |value: i32| {
                        on_change_fov(FieldOfView {
                            angle_deg: value as u16,
                            ..fov.clone()
                        });
                    }
                },
            }

            FovSlider {
                label: "Range",
                unit: "cm",
                min: 50,
                max: 5000,
                value: fov.range,
                on_preview: {
                    let fov = fov.clone();
                    move |value: i32| {
                        on_preview_fov(FieldOfView {
                            range: value,
                            ..fov.clone()
                        });
                    }
                },
                on_change: {
                    let fov = fov.clone();
                    move |value: i32| {
                        on_change_fov(FieldOfView {
                            range: value,
                            ..fov.clone()
                        });
                    }
                },
            }
        }
    }
}

/// A labelled range slider. Fires `on_preview` on every move (`oninput`) for
/// live feedback and `on_change` once on release (`onchange`) for the committed
/// value, so dragging never floods the undo stack.
#[component]
fn FovSlider(
    label: String,
    unit: String,
    min: i32,
    max: i32,
    value: i32,
    on_preview: Callback<i32>,
    on_change: Callback<i32>,
) -> Element {
    rsx! {
        label { class: "camera-inspector__field",
            div { class: "camera-inspector__field-label",
                span { "{label}" }
                span { class: "camera-inspector__field-value", "{value}{unit}" }
            }
            input {
                r#type: "range",
                min: "{min}",
                max: "{max}",
                value: "{value}",
                oninput: move |evt| {
                    if let Ok(parsed) = evt.value().parse::<i32>() {
                        on_preview(parsed);
                    }
                },
                onchange: move |evt| {
                    if let Ok(parsed) = evt.value().parse::<i32>() {
                        on_change(parsed);
                    }
                },
            }
        }
    }
}
