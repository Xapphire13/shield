use dioxus::prelude::*;

use crate::components::MapView;

#[component]
pub fn Map() -> Element {
    rsx! {
        MapView {}
    }
}
