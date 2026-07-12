use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{
    LdBrickWall, LdCheck, LdDoorOpen, LdMousePointer, LdVideo, LdX,
};
use shield_models::Camera;

use crate::components::map::interaction::Tool;

stylance::import_crate_style!(style, "src/components/map/edit_toolbar.module.css");

/// Bottom tool strip shown while editing the map. It is positioned in the same
/// bottom zone as the global navigation toolbar and stacks above it, visually
/// replacing it for the duration of edit mode.
///
/// Icon-only buttons (a `title` gives a hover tooltip / accessible name, but no
/// visible text label — this strip is expected to grow more tools in later
/// PRs, and labels would not fit). Takes the host's `Tool` directly (rather
/// than one bool per tool) so each new tool button just matches a new
/// variant instead of the caller needing to pre-compute and wire up another
/// boolean.
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
    /// Arm the Draw-Wall tool.
    on_draw_wall: Callback,
    /// Finish the in-progress wall draft as an open path.
    on_finish_wall: Callback,
    /// Arm the Place-Door tool.
    on_place_door: Callback,
) -> Element {
    let camera_active = camera_picker_open || matches!(active_tool, Tool::PlaceCamera(_));
    let wall_active = matches!(active_tool, Tool::DrawWall { .. });
    let can_finish_wall =
        matches!(&active_tool, Tool::DrawWall { vertices } if vertices.len() >= 2);
    let door_active = matches!(active_tool, Tool::PlaceDoor { .. });
    rsx! {
        div { class: style::container,
            button {
                class: style::tool,
                "data-active": matches!(active_tool, Tool::Select),
                title: "Select",
                onclick: move |_| on_select(()),
                Icon { width: 20, height: 20, icon: LdMousePointer }
            }
            button {
                class: style::tool,
                "data-active": camera_active,
                title: "Place camera",
                onclick: move |_| on_add_camera(()),
                Icon { width: 20, height: 20, icon: LdVideo }
            }
            button {
                class: style::tool,
                "data-active": wall_active,
                title: "Draw wall",
                onclick: move |_| on_draw_wall(()),
                Icon { width: 20, height: 20, icon: LdBrickWall }
            }
            button {
                class: style::tool,
                "data-active": door_active,
                title: "Place door",
                onclick: move |_| on_place_door(()),
                Icon { width: 20, height: 20, icon: LdDoorOpen }
            }
            if can_finish_wall {
                button {
                    class: "{style::tool} {style::labeled}",
                    title: "Finish wall",
                    onclick: move |_| on_finish_wall(()),
                    Icon { width: 20, height: 20, icon: LdCheck }
                    span { "Done" }
                }
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
        div { class: style::picker,
            div { class: style::picker_header,
                span { class: style::picker_title, "Add a camera" }
                button {
                    class: style::picker_close,
                    onclick: move |_| on_close(()),
                    Icon { width: 20, height: 20, icon: LdX }
                }
            }

            if cameras.is_empty() {
                div { class: style::picker_empty, "All cameras are already on the map." }
            } else {
                div { class: style::picker_list,
                    for camera in cameras.iter().cloned() {
                        button {
                            key: "{camera.id}",
                            class: style::picker_item,
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
