use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdCheck, LdMinus, LdMousePointer, LdVideo, LdX};
use shield_models::Camera;

/// Bottom tool strip shown while editing the map. It is positioned in the same
/// bottom zone as the global navigation toolbar and stacks above it, visually
/// replacing it for the duration of edit mode.
///
/// Icon-only buttons (a `title` gives a hover tooltip / accessible name, but no
/// visible text label — this strip is expected to grow more tools, like
/// draw-wall and place-door, in later PRs, and labels would not fit). The host
/// (`MapView`) owns which tool is active, tracked as a private enum there; this
/// component only sees plain booleans/callbacks, so it stays decoupled from
/// that internal shape.
#[component]
pub fn EditToolbar(
    /// Whether the Select tool is currently active (the neutral default).
    select_active: bool,
    /// Whether Place-Camera is armed (either the picker sheet is open, or a
    /// specific camera has been chosen and is awaiting a placement tap).
    camera_active: bool,
    /// Switch to the Select tool (also used to cancel an in-progress camera
    /// placement).
    on_select: Callback,
    /// Open the camera picker sheet (existing "Add camera" behavior).
    on_add_camera: Callback,
    /// Whether Draw-Wall is currently active (a wall draft is in progress, or
    /// just armed with no vertices placed yet).
    wall_active: bool,
    /// Arm the Draw-Wall tool.
    on_draw_wall: Callback,
    /// Shown only while a wall draft has enough vertices to finish as an open
    /// path (>= 2).
    can_finish_wall: bool,
    /// Finish the in-progress wall draft as an open path.
    on_finish_wall: Callback,
) -> Element {
    rsx! {
        div { class: "edit-toolbar",
            button {
                class: "edit-toolbar__tool",
                "data-active": select_active,
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
            button {
                class: "edit-toolbar__tool",
                "data-active": wall_active,
                title: "Draw wall",
                onclick: move |_| on_draw_wall(()),
                Icon { width: 20, height: 20, icon: LdMinus }
            }
            if can_finish_wall {
                button {
                    class: "edit-toolbar__tool edit-toolbar__tool--labeled",
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
