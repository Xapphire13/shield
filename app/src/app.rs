use dioxus::prelude::*;

use crate::{
    components::{BottomToolbar, MainView},
    hooks::use_api_client_provider,
    pages::{Home, Login, Map, NotFound},
    utils::navigate_to,
};

#[derive(Routable, Clone)]
#[rustfmt::skip]
pub enum Route {
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
    let handle_on_unauthorized = move || {
        // The API client lives here, above the `Router`, so that `Login` and the
        // main views all share it. That means this callback has no router context
        // and can't use `navigator()` — hence the raw location-based redirect.
        let _ = navigate_to("/login");
    };
    use_api_client_provider(handle_on_unauthorized);

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
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
