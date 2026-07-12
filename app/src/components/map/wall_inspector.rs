use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::LdTrash2;
use shield_models::{MapWall, WallColor};

use crate::components::map::color_swatch_picker::ColorSwatchPicker;
use crate::components::ui::{ButtonColor, IconButton};

stylance::import_crate_style!(style, "src/components/map/wall_inspector.module.css");

/// Contextual bottom sheet for the currently selected wall. Vertex
/// repositioning happens via the on-canvas handles (see `MapWallPath`); this
/// sheet exposes actions that don't have a natural on-canvas gesture:
/// recoloring, closing an open path into a loop, and deleting the wall.
#[component]
pub fn WallInspector(
    wall: MapWall,
    /// Close this wall's path into a loop (only offered once there are
    /// enough vertices for a loop to make sense).
    on_close_loop: Callback,
    on_recolor: Callback<WallColor>,
    on_delete: Callback,
) -> Element {
    rsx! {
        div { class: style::container,
            div { class: style::header,
                div { class: style::title, "Wall" }
                IconButton {
                    icon: rsx! {
                        Icon { width: 20, height: 20, icon: LdTrash2 }
                    },
                    color: ButtonColor::Danger,
                    on_press: move |_| on_delete(()),
                }
            }
            ColorSwatchPicker { value: wall.color, on_change: on_recolor }
            if !wall.closed && wall.vertices.len() >= 3 {
                button {
                    class: style::close_loop,
                    onclick: move |_| on_close_loop(()),
                    "Close loop"
                }
            }
        }
    }
}
