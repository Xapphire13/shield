use dioxus::prelude::*;
use dioxus_feather_icons::icon;

/// The primary views the user can switch between from the bottom toolbar.
#[derive(Clone, Copy, PartialEq)]
pub enum MainView {
    List,
    Map,
}

#[component]
pub fn BottomToolbar(view: MainView, on_change: Callback<MainView>) -> Element {
    rsx! {
        nav { class: "bottom-toolbar",
            button {
                class: "bottom-toolbar__item",
                "data-active": view == MainView::List,
                onclick: move |_| on_change.call(MainView::List),
                {icon!(list)}
                span { "List" }
            }

            button {
                class: "bottom-toolbar__item",
                "data-active": view == MainView::Map,
                onclick: move |_| on_change.call(MainView::Map),
                {icon!(map)}
                span { "Map" }
            }
        }
    }
}
