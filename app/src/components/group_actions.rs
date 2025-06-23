use dioxus::prelude::*;
use dioxus_feather_icons::icon;

use crate::components::ui::{ButtonColor, IconButton};

#[component]
pub fn GroupActions() -> Element {
    rsx! {
        div { class: "group-actions",
            IconButton { icon: icon!(video), color: ButtonColor::Danger }

            IconButton { icon: icon!(video_off) }
        }
    }
}
