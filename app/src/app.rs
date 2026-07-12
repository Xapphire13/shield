use dioxus::prelude::*;

use crate::{
    components::{BottomToolbar, CameraList, MapView},
    hooks::use_api_client_provider,
    pages::{Login, NotFound},
};

#[derive(Routable, Clone)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppRoot)]
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

/// Hand-written global styles: root variables, resets, and shared layout.
const GLOBAL_CSS: Asset = asset!("/assets/global.css");
/// Scoped component styles bundled from the co-located `*.module.css` files by
/// the stylance CLI (gitignored; `scripts/dev.sh` regenerates it on save).
const STYLES_CSS: Asset = asset!("/assets/styles.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: GLOBAL_CSS }
        document::Link { rel: "stylesheet", href: STYLES_CSS }

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
