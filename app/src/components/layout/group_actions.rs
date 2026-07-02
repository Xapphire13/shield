use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdVideo, LdVideoOff};

use crate::components::ui::{ButtonColor, IconButton};

#[component]
pub fn GroupActions(on_toggle_record_on: Callback, on_toggle_record_off: Callback) -> Element {
    rsx! {
        div { class: "group-actions",
            IconButton {
                icon: rsx! {
                    Icon { width: 24, height: 24, icon: LdVideo }
                },
                color: ButtonColor::Danger,
                on_press: on_toggle_record_on,
            }

            IconButton {
                icon: rsx! {
                    Icon { width: 24, height: 24, icon: LdVideoOff }
                },
                on_press: on_toggle_record_off,
            }
        }
    }
}
