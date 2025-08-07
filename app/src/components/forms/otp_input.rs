use dioxus::prelude::*;

use crate::utils::focus_element;

#[component]
pub fn OtpInput(
    value: String,
    on_change: Callback<String>,
    on_submit: Option<Callback>,
) -> Element {
    let mut digits = value.chars().collect::<Vec<_>>();
    // Pad with empty chars if needed
    digits.resize(6, '\0');

    let create_handler = move |digit_index: usize| {
        let on_change = on_change;
        let on_submit = on_submit;
        let current_value = value.clone();

        Callback::new(move |ev: FormEvent| {
            if let Some(new_digit) = ev.value().chars().next() {
                if new_digit.is_ascii_digit() {
                    let mut new_digits: Vec<char> = current_value.chars().collect();
                    new_digits.resize(6, '\0');
                    new_digits[digit_index] = new_digit;

                    let new_value: String = new_digits.iter().filter(|&&c| c != '\0').collect();
                    on_change.call(new_value.clone());

                    let next_digit = new_value.len();
                    if next_digit >= 6 {
                        if let Some(submit) = on_submit {
                            submit.call(());
                        }
                    } else if next_digit < 6 {
                        let next_input_id = format!("otp-digit-{}", next_digit + 1);
                        let _ = focus_element(&next_input_id);
                    }
                }
            }
        })
    };

    rsx! {
        div { class: "otp-input-container",
            input {
                id: "otp-digit-1",
                r#type: "text",
                maxlength: "1",
                "data-bwignore": "true",
                autocomplete: "off",
                oninput: create_handler(0),
                value: if !digits.is_empty() && digits[0] != '\0' { digits[0].to_string() } else { String::new() },
            }
            input {
                id: "otp-digit-2",
                r#type: "text",
                maxlength: "1",
                "data-bwignore": "true",
                autocomplete: "off",
                oninput: create_handler(1),
                value: if digits.len() > 1 && digits[1] != '\0' { digits[1].to_string() } else { String::new() },
            }
            input {
                id: "otp-digit-3",
                r#type: "text",
                maxlength: "1",
                "data-bwignore": "true",
                autocomplete: "off",
                oninput: create_handler(2),
                value: if digits.len() > 2 && digits[2] != '\0' { digits[2].to_string() } else { String::new() },
            }
            input {
                id: "otp-digit-4",
                r#type: "text",
                maxlength: "1",
                "data-bwignore": "true",
                autocomplete: "off",
                oninput: create_handler(3),
                value: if digits.len() > 3 && digits[3] != '\0' { digits[3].to_string() } else { String::new() },
            }
            input {
                id: "otp-digit-5",
                r#type: "text",
                maxlength: "1",
                "data-bwignore": "true",
                autocomplete: "off",
                oninput: create_handler(4),
                value: if digits.len() > 4 && digits[4] != '\0' { digits[4].to_string() } else { String::new() },
            }
            input {
                id: "otp-digit-6",
                r#type: "text",
                maxlength: "1",
                "data-bwignore": "true",
                autocomplete: "off",
                oninput: create_handler(5),
                value: if digits.len() > 5 && digits[5] != '\0' { digits[5].to_string() } else { String::new() },
            }
        }
    }
}
