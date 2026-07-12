use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::LdTrash2;
use shield_models::MapDoor;

use crate::components::ui::{ButtonColor, IconButton};

stylance::import_crate_style!(style, "src/components/map/door_inspector.module.css");

/// Contextual bottom sheet for the currently selected door. Endpoint
/// repositioning happens via the on-canvas handles (see `MapDoorMarker`);
/// this sheet exposes the one action without a natural on-canvas gesture —
/// flipping which side the door swings toward — plus delete.
#[component]
pub fn DoorInspector(
    door: MapDoor,
    /// Flip the door's swing side (`Left` <-> `Right`), committed
    /// immediately — a binary state has no meaningful in-between to preview.
    on_flip_swing: Callback,
    on_delete: Callback,
) -> Element {
    rsx! {
        div { class: style::container,
            div { class: style::header,
                div { class: style::title, "Door" }
                IconButton {
                    icon: rsx! {
                        Icon { width: 20, height: 20, icon: LdTrash2 }
                    },
                    color: ButtonColor::Danger,
                    on_press: move |_| on_delete(()),
                }
            }
            button {
                class: style::flip_swing,
                onclick: move |_| on_flip_swing(()),
                "Flip swing (currently {door.swing:?})"
            }
        }
    }
}
