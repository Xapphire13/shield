use dioxus::prelude::*;
use dioxus_feather_icons::icon;

use crate::app::Route;

#[component]
pub fn BottomToolbar() -> Element {
    rsx! {
        nav { class: "bottom-toolbar",
            Link {
                to: Route::CameraList,
                class: "bottom-toolbar__item",
                active_class: "bottom-toolbar__item--active",
                {icon!(list)}
                span { "List" }
            }

            Link {
                to: Route::MapView,
                class: "bottom-toolbar__item",
                active_class: "bottom-toolbar__item--active",
                {icon!(map)}
                span { "Map" }
            }
        }
    }
}
