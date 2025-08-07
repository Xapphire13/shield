use crate::{
    api::AuthApi,
    app::Route,
    components::{PrimaryButton, otp_input::OtpInput},
    hooks::use_api_client,
};
use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let nav = navigator();
    let client = use_api_client();
    let mut code = use_signal(String::new);

    let handle_code_changed = use_callback(move |new_code| code.set(new_code));

    let handle_submit = use_callback(move |_| {
        let client = client.clone();
        spawn(async move {
            match client.authenticate(code()).await {
                Ok(_) => {
                    nav.replace(Route::Home);
                }
                Err(_) => {
                    // TODO
                }
            }
        });
    });

    rsx! {
        div { class: "login-container",
            div { class: "login-card",
                div { class: "otp-heading", "Enter OTP Code" }
                OtpInput { value: "", on_change: handle_code_changed, on_submit: handle_submit }
                PrimaryButton { id: "otp-submit-button", on_press: handle_submit, "Submit" }
            }
        }
    }
}
