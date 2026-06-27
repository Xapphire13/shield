use crate::{
    api::AuthApi,
    app::Route,
    components::{
        PrimaryButton,
        otp_input::{OtpInput, code_is_filled_out},
    },
    hooks::use_api_client,
};
use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let nav = navigator();
    let client = use_api_client();
    let mut code = use_signal(|| ['\0', '\0', '\0', '\0', '\0', '\0']);
    let mut loading = use_signal(|| false);

    let handle_code_changed = use_callback(move |new_code| code.set(new_code));

    let handle_submit = use_callback(move |_| {
        loading.set(true);
        let client = client.clone();
        spawn(async move {
            match client.authenticate(code().iter().collect()).await {
                Ok(_) => {
                    nav.replace(Route::CameraList);
                }
                Err(_) => {
                    // TODO
                }
            }

            loading.set(false);
        });
    });

    rsx! {
        div { class: "login-container",
            div { class: "login-card",
                div { class: "otp-heading", "Enter OTP Code" }
                OtpInput {
                    value: code(),
                    on_change: handle_code_changed,
                    on_submit: handle_submit,
                }
                PrimaryButton {
                    on_press: handle_submit,
                    disabled: loading() || !code_is_filled_out(code()),
                    if loading() {
                        "Loading..."
                    } else {
                        "Submit"
                    }
                }
            }
        }
    }
}
