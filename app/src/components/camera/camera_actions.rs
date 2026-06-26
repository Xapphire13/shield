use dioxus::prelude::*;
use dioxus_feather_icons::icon;

use crate::components::ui::{ButtonColor, IconButton};

#[component]
pub fn CameraActions(
    visible: bool,
    on_toggle_record_on: Callback,
    on_toggle_record_off: Callback,
    on_dismiss: Callback,
) -> Element {
    rsx! {
        div { class: "camera-actions", "data-visible": visible, inert: (!visible).then_some(true),
            IconButton {
                icon: icon!(video),
                color: ButtonColor::Danger,
                on_press: on_toggle_record_on,
            }

            IconButton { icon: icon!(video_off), on_press: on_toggle_record_off }

            div { class: "camera-actions__separator" }

            IconButton { icon: icon!(x), on_press: on_dismiss }
        }
    }
}
