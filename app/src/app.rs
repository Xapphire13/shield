use dioxus::prelude::*;

use crate::{
    components::{BottomToolbar, CameraList, MapView},
    hooks::use_api_client_provider,
    pages::{Login, NotFound},
};

#[derive(Routable, Clone)]
#[rustfmt::skip]
pub enum Route {
    // `AppRoot` provides the API client to every route from inside the
    // `Router`, so router-aware navigation is available app-wide.
    #[layout(AppRoot)]
        // Primary views share the bottom toolbar via the `MainShell` layout.
        #[layout(MainShell)]
            #[route("/")]
            CameraList,
            #[route("/map")]
            MapView,
        #[end_layout]
        #[route("/login")]
        Login,
        #[route("/:..route")]
        NotFound { route: Vec<String> },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}

/// Root layout for every route. Provides the shared API client and, because it
/// renders inside the `Router`, lets the unauthorized handler redirect via the
/// router navigator.
#[component]
fn AppRoot() -> Element {
    let nav = navigator();
    use_api_client_provider(move || {
        nav.replace(Route::Login);
    });

    rsx! {
        Outlet::<Route> {}
    }
}

/// Layout wrapping the primary views: renders the active view through the
/// `Outlet` with a persistent bottom toolbar for switching between them.
#[component]
fn MainShell() -> Element {
    rsx! {
        Outlet::<Route> {}

        BottomToolbar {}
    }
}
