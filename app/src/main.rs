use dioxus::prelude::*;

mod components;
mod pages;
mod use_update_recording_mode;

use pages::Home;
use pages::Login;

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
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}

fn get_api_url(path: &str) -> String {
    let hostname = web_sys::window().unwrap().location().hostname().unwrap();

    format!("http://{hostname}:3000{path}")
}
