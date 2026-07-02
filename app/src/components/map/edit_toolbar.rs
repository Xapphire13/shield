use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdPlus, LdX};
use shield_models::Camera;

/// Bottom tool strip shown while editing the map. It is positioned in the same
/// bottom zone as the global navigation toolbar and stacks above it, visually
/// replacing it for the duration of edit mode.
///
/// Its only tool in v1 is **Add camera**, which opens a picker of cameras not
/// yet placed on the map.
#[component]
pub fn EditToolbar(on_add: Callback) -> Element {
    rsx! {
        div { class: "edit-toolbar",
            button {
                class: "edit-toolbar__tool",
                onclick: move |_| on_add(()),
                Icon { width: 20, height: 20, icon: LdPlus }
                span { "Add camera" }
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
