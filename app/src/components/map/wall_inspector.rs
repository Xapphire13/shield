use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::LdTrash2;
use shield_models::MapWall;

use crate::components::ui::{ButtonColor, IconButton};

/// Contextual bottom sheet for the currently selected wall. Vertex
/// repositioning happens via the on-canvas handles (see `MapWallPath`); this
/// sheet exposes actions that don't have a natural on-canvas gesture: closing
/// an open path into a loop, and deleting the wall. Recoloring lands in a
/// later PR alongside the color palette.
#[component]
pub fn WallInspector(
    wall: MapWall,
    /// Close this wall's path into a loop (only offered once there are
    /// enough vertices for a loop to make sense).
    on_close_loop: Callback,
    on_delete: Callback,
) -> Element {
    rsx! {
        div { class: "wall-inspector",
            div { class: "wall-inspector__header",
                div { class: "wall-inspector__title", "Wall" }
                IconButton {
                    icon: rsx! {
                        Icon { width: 20, height: 20, icon: LdTrash2 }
                    },
                    color: ButtonColor::Danger,
                    on_press: move |_| on_delete(()),
                }
            }
            if !wall.closed && wall.vertices.len() >= 3 {
                button {
                    class: "wall-inspector__close-loop",
                    onclick: move |_| on_close_loop(()),
                    "Close loop"
                }
            }
        }
    }
}
