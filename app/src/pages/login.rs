use dioxus::prelude::*;
use web_sys::{HtmlElement, wasm_bindgen::JsCast, window};

use crate::{Route, components::ui::PrimaryButton, use_api_client::use_api_client};

fn focus_element(id: &str) {
    window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|el| el.dyn_into::<HtmlElement>().ok())
        .and_then(|input| input.focus().ok());
}

#[component]
pub fn Login() -> Element {
    let nav = navigator();
    let client = use_api_client();
    let mut code = use_signal(String::new);

    let handle_input = |digit: u8| {
        move |ev: FormEvent| {
            if let Ok(value) = ev.value().parse::<u32>() {
                let prev: String = code()
                    .chars()
                    .take(digit.saturating_sub(1) as usize)
                    .collect();
                let next: String = format!("{prev}{value}").chars().take(6).collect();
                let next_digit = next.len() + 1;

                code.set(next);

                if next_digit > 6 {
                    focus_element("otp-submit-button");
                } else {
                    let next_input_id = format!("otp-digit-{next_digit}");
                    focus_element(&next_input_id);
                }
            }
        }
    };

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

    let code = code();
    let mut digits = code.chars();

    rsx! {
        div { class: "login-container",
            div { class: "login-card",
                div { class: "otp-heading", "Enter OTP Code" }
                div { class: "otp-input-container",
                    input {
                        id: "otp-digit-1",
                        r#type: "text",
                        "data-bwignore": "true",
                        autocomplete: "off",
                        oninput: handle_input(1),
                        value: digits.next().map(|c| c.to_string()).unwrap_or(String::new()),
                    }
                    input {
                        id: "otp-digit-2",
                        r#type: "text",
                        "data-bwignore": "true",
                        autocomplete: "off",
                        oninput: handle_input(2),
                        value: digits.next().map(|c| c.to_string()).unwrap_or(String::new()),
                    }
                    input {
                        id: "otp-digit-3",
                        r#type: "text",
                        "data-bwignore": "true",
                        autocomplete: "off",
                        oninput: handle_input(3),
                        value: digits.next().map(|c| c.to_string()).unwrap_or(String::new()),
                    }
                    input {
                        id: "otp-digit-4",
                        r#type: "text",
                        "data-bwignore": "true",
                        autocomplete: "off",
                        oninput: handle_input(4),
                        value: digits.next().map(|c| c.to_string()).unwrap_or(String::new()),
                    }
                    input {
                        id: "otp-digit-5",
                        r#type: "text",
                        "data-bwignore": "true",
                        autocomplete: "off",
                        oninput: handle_input(5),
                        value: digits.next().map(|c| c.to_string()).unwrap_or(String::new()),
                    }
                    input {
                        id: "otp-digit-6",
                        r#type: "text",
                        "data-bwignore": "true",
                        autocomplete: "off",
                        oninput: handle_input(6),
                        value: digits.next().map(|c| c.to_string()).unwrap_or(String::new()),
                    }
                }
                PrimaryButton { id: "otp-submit-button", on_press: handle_submit, "Submit" }
            }
        }
    }
}
