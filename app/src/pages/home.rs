use dioxus::prelude::*;

use crate::components::{BottomToolbar, CameraList, MainView, MapView};

#[component]
pub fn Home() -> Element {
    let mut view = use_signal(|| MainView::List);

    rsx! {
        match view() {
            MainView::List => rsx! {
                CameraList {}
            },
            MainView::Map => rsx! {
                MapView {}
            },
        }

        BottomToolbar { view: view(), on_change: move |next| view.set(next) }
    }
}
