use dioxus::prelude::*;

use crate::components::CameraList;

#[component]
pub fn Home() -> Element {
    rsx! {
        CameraList {}
    }
}
