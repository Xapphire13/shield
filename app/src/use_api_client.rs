use std::rc::Rc;

use dioxus::prelude::*;

use crate::api_client::ApiClient;

#[cfg(debug_assertions)]
const BASE_URL: &str = "http://{}:3000";
#[cfg(not(debug_assertions))]
const BASE_URL: &str = "http://{}/api";

pub fn use_api_client_provider(on_unauthorized: impl Fn() + 'static) {
    use_context_provider(|| {
        let hostname = web_sys::window().unwrap().location().hostname().unwrap();
        let base_url = BASE_URL.replace("{}", &hostname);
        let client = ApiClient::new(base_url, on_unauthorized);

        Rc::new(client)
    });
}

pub fn use_api_client() -> Rc<ApiClient> {
    use_context::<Rc<ApiClient>>()
}
