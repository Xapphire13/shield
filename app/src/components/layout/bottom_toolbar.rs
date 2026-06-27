use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::fi_icons::{FiList, FiMap};

use crate::app::Route;

#[component]
pub fn BottomToolbar() -> Element {
    rsx! {
        nav { class: "bottom-toolbar",
            Link {
                to: Route::CameraList,
                class: "bottom-toolbar__item",
                active_class: "bottom-toolbar__item--active",
                Icon { width: 20, height: 20, icon: FiList }
                span { "List" }
            }

            Link {
                to: Route::MapView,
                class: "bottom-toolbar__item",
                active_class: "bottom-toolbar__item--active",
                Icon { width: 20, height: 20, icon: FiMap }
                span { "Map" }
            }
        }
    }
}
