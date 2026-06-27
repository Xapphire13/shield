use dioxus::prelude::*;

use crate::{
    hooks::use_api_client_provider,
    pages::{Home, Login, NotFound},
    utils::navigate_to,
};

#[derive(Routable, Clone)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    Home,
    #[route("/login")]
    Login,
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    let handle_on_unauthorized = move || {
        let _ = navigate_to("/login");
    };
    use_api_client_provider(handle_on_unauthorized);

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}
