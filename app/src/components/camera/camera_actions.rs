use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdVideo, LdVideoOff, LdX};

use crate::components::ui::{ButtonColor, IconButton};

stylance::import_crate_style!(style, "src/components/camera/camera_actions.module.css");

#[component]
pub fn CameraActions(
    visible: bool,
    on_toggle_record_on: Callback,
    on_toggle_record_off: Callback,
    on_dismiss: Callback,
) -> Element {
    rsx! {
        div { class: style::container, "data-visible": visible, inert: (!visible).then_some(true),
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

            div { class: style::separator }

            IconButton {
                icon: rsx! {
                    Icon { width: 24, height: 24, icon: LdX }
                },
                on_press: on_dismiss,
            }
        }
    }
}
