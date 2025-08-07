mod api;
mod app;
mod components;
mod hooks;
mod pages;
mod storage;
mod types;
mod utils;

use crate::app::App;

fn main() {
    dioxus::launch(App);
}
