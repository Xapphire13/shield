use dioxus::prelude::*;
use dioxus_feather_icons::icon;

use crate::components::ui::{ButtonColor, IconButton};

#[component]
pub fn GroupActions(on_toggle_record_on: Callback, on_toggle_record_off: Callback) -> Element {
    rsx! {
        div { class: "group-actions",
            IconButton {
                icon: icon!(video),
                color: ButtonColor::Danger,
                on_press: on_toggle_record_on,
            }

            IconButton { icon: icon!(video_off), on_press: on_toggle_record_off }
        }
    }
}
