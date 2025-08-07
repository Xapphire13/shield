use std::rc::Rc;

use dioxus::prelude::*;

use crate::api_client::ApiClient;

pub fn use_api_client_provider(on_unauthorized: impl Fn() + 'static) {
    use_context_provider(|| {
        let hostname = web_sys::window().unwrap().location().hostname().unwrap();
        let base_url = format!("http://{hostname}:3000");
        let client = ApiClient::new(base_url, on_unauthorized);

        Rc::new(client)
    });
}

pub fn use_api_client() -> Rc<ApiClient> {
    use_context::<Rc<ApiClient>>()
}
