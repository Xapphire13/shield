use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdList, LdMap};

use crate::app::Route;

stylance::import_crate_style!(style, "src/components/layout/bottom_toolbar.module.css");

#[component]
pub fn BottomToolbar() -> Element {
    rsx! {
        nav { class: style::container,
            Link {
                to: Route::CameraList,
                class: style::item,
                active_class: style::active,
                Icon { width: 20, height: 20, icon: LdList }
                span { "List" }
            }

            Link {
                to: Route::MapView,
                class: style::item,
                active_class: style::active,
                Icon { width: 20, height: 20, icon: LdMap }
                span { "Map" }
            }
        }
    }
}
