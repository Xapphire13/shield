use dioxus::prelude::*;

mod api_client;
mod components;
mod pages;
mod token_store;
mod use_api_client;
mod use_cameras;
mod use_update_recording_mode;

use pages::{Home, Login, NotFound};

use crate::use_api_client::use_api_client_provider;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[derive(Routable, Clone)]
enum Route {
    #[route("/")]
    Home,
    #[route("/login")]
    Login,
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

#[component]
fn App() -> Element {
    let handle_on_unauthorized = move || {
        web_sys::window()
            .unwrap()
            .location()
            .replace("/login")
            .unwrap();
    };
    use_api_client_provider(handle_on_unauthorized);

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}

        {dioxus_feather_icons::sprite!()}
    }
}
