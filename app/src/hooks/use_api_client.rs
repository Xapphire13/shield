use std::rc::Rc;

use dioxus::prelude::*;

use crate::{api::ApiClient, utils::get_hostname};

#[cfg(debug_assertions)]
pub const BASE_URL: &str = "http://{}:3000";
#[cfg(not(debug_assertions))]
pub const BASE_URL: &str = "https://{}/api";

pub fn use_api_client_provider(on_unauthorized: impl Fn() + 'static) {
    use_context_provider(|| {
        let hostname = get_hostname().unwrap();
        let base_url = BASE_URL.replace("{}", &hostname);
        let client = ApiClient::new(base_url, on_unauthorized);

        Rc::new(client)
    });
}

pub fn use_api_client() -> Rc<ApiClient> {
    use_context::<Rc<ApiClient>>()
}
