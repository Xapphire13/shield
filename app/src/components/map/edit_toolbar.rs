use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdMousePointer, LdVideo, LdX};
use shield_models::Camera;

use crate::components::map::map_view::Tool;

/// Bottom tool strip shown while editing the map. It is positioned in the same
/// bottom zone as the global navigation toolbar and stacks above it, visually
/// replacing it for the duration of edit mode.
///
/// Icon-only buttons (a `title` gives a hover tooltip / accessible name, but no
/// visible text label — this strip is expected to grow more tools, like
/// draw-wall and place-door, in later PRs, and labels would not fit). Takes
/// the host's `Tool` directly (rather than one bool per tool) so each new
/// tool button just matches a new variant instead of the caller needing to
/// pre-compute and wire up another boolean.
#[component]
pub fn EditToolbar(
    /// The currently active tool; each button's active state is derived from
    /// this by matching its variant.
    active_tool: Tool,
    /// Whether the camera picker sheet is open. Not part of `Tool` (picking a
    /// camera happens before `active_tool` becomes `PlaceCamera`), so it's
    /// passed alongside to keep the Place-Camera button highlighted while the
    /// sheet is up.
    camera_picker_open: bool,
    /// Switch to the Select tool (also used to cancel an in-progress camera
    /// placement).
    on_select: Callback,
    /// Open the camera picker sheet (existing "Add camera" behavior).
    on_add_camera: Callback,
) -> Element {
    let camera_active = camera_picker_open || matches!(active_tool, Tool::PlaceCamera(_));
    rsx! {
        div { class: "edit-toolbar",
            button {
                class: "edit-toolbar__tool",
                "data-active": matches!(active_tool, Tool::Select),
                title: "Select",
                onclick: move |_| on_select(()),
                Icon { width: 20, height: 20, icon: LdMousePointer }
            }
            button {
                class: "edit-toolbar__tool",
                "data-active": camera_active,
                title: "Place camera",
                onclick: move |_| on_add_camera(()),
                Icon { width: 20, height: 20, icon: LdVideo }
            }
        }
    }
}

/// Bottom sheet listing cameras that can still be placed on the map. Selecting
/// one hands its id back to the host, which then waits for the next canvas tap
/// to drop it at those world coordinates.
#[component]
pub fn CameraPicker(
    /// Cameras not yet placed on the map.
    cameras: Vec<Camera>,
    /// A camera was chosen; carries its id.
    on_pick: Callback<String>,
    /// The sheet was dismissed without choosing.
    on_close: Callback,
) -> Element {
    rsx! {
        div { class: "camera-picker",
            div { class: "camera-picker__header",
                span { class: "camera-picker__title", "Add a camera" }
                button {
                    class: "camera-picker__close",
                    onclick: move |_| on_close(()),
                    Icon { width: 20, height: 20, icon: LdX }
                }
            }

            if cameras.is_empty() {
                div { class: "camera-picker__empty", "All cameras are already on the map." }
            } else {
                div { class: "camera-picker__list",
                    for camera in cameras.iter().cloned() {
                        button {
                            key: "{camera.id}",
                            class: "camera-picker__item",
                            onclick: {
                                let id = camera.id.clone();
                                move |_| on_pick(id.clone())
                            },
                            "{camera.name}"
                        }
                    }
                }
            }
        }
    }
}
