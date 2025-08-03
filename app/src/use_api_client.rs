use dioxus::prelude::*;

use crate::api_client::ApiClient;

pub fn use_api_client_provider(on_not_authorized: impl Fn() + 'static) {
    use_context_provider(|| {
        let client = ApiClient::new(on_not_authorized);

        match client {
            Ok(client) => Signal::new(Some(client)),
            Err(_) => Signal::new(None),
        }
    });
}

pub fn use_api_client() -> Signal<Option<ApiClient>> {
    use_context::<Signal<Option<ApiClient>>>()
}
