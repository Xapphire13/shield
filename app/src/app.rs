use dioxus::prelude::*;

use crate::{
    components::{BottomToolbar, MainView},
    hooks::use_api_client_provider,
    pages::{Home, Login, Map, NotFound},
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
            Home,
            #[route("/map")]
            Map,
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
    let view = match use_route::<Route>() {
        Route::Map => MainView::Map,
        _ => MainView::List,
    };
    let nav = navigator();

    rsx! {
        Outlet::<Route> {}

        BottomToolbar {
            view,
            on_change: move |next| {
                match next {
                    MainView::List => nav.push(Route::Home),
                    MainView::Map => nav.push(Route::Map),
                };
            },
        }
    }
}
