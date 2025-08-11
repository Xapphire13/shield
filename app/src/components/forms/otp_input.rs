use dioxus::{prelude::*, web::WebEventExt};
use web_sys::wasm_bindgen::JsCast;

use crate::utils::focus_element;

#[component]
pub fn OtpInput(
    value: [char; 6],
    on_change: Callback<[char; 6]>,
    on_submit: Option<Callback>,
) -> Element {
    let create_handlers = move |digit_pos: usize| {
        let handle_key_down = Callback::new(move |ev: KeyboardEvent| match ev.key() {
            Key::Backspace => {
                let digit_index = digit_pos - 1;
                let mut new_value = value;

                if new_value[digit_index] == '\0' && digit_pos > 1 {
                    let prev_digit_pos = digit_pos - 1;
                    new_value[digit_index - 1] = '\0';
                    let prev_input_id = format!("otp-digit-{prev_digit_pos}");
                    let _ = focus_element(&prev_input_id);
                } else {
                    new_value[digit_index] = '\0';
                }

                on_change(new_value);

                ev.prevent_default();
                ev.stop_propagation();
            }
            Key::Character(char) if !ev.modifiers().meta() && !ev.modifiers().ctrl() => {
                if let Some(new_digit) = char.chars().next()
                    && new_digit.is_numeric()
                {
                    let mut new_value = value;
                    let next_digit_pos = apply_code_update(&mut new_value, &char, digit_pos);

                    if next_digit_pos != digit_pos {
                        on_change(new_value);

                        if next_digit_pos > 6 && new_value.iter().all(|digit| digit.is_numeric()) {
                            if let Some(submit) = on_submit {
                                submit(());
                            }
                        } else if next_digit_pos <= 6 {
                            focus_on_input(next_digit_pos);
                        }
                    }
                }

                ev.prevent_default();
                ev.stop_propagation();
            }
            _ => {}
        });

        let handle_paste = Callback::new(move |ev: ClipboardEvent| {
            if let Ok(clipboard_event) = ev.as_web_event().dyn_into::<web_sys::ClipboardEvent>() {
                if let Ok(text) = clipboard_event.clipboard_data().unwrap().get_data("text") {
                    let mut new_value = value;
                    let next_digit_pos = apply_code_update(&mut new_value, &text, digit_pos);

                    if next_digit_pos != digit_pos {
                        on_change(new_value);

                        if next_digit_pos > 6 && code_is_filled_out(new_value) {
                            if let Some(submit) = on_submit {
                                submit(());
                            }
                        } else if next_digit_pos <= 6 {
                            focus_on_input(next_digit_pos);
                        }
                    }
                }

                ev.stop_propagation();
                ev.prevent_default();
            }
        });

        (handle_key_down, handle_paste)
    };

    rsx! {
        div { class: "otp-input-container",
            {
                value
                    .iter()
                    .enumerate()
                    .map(|(i, &digit)| {
                        let position = i + 1;
                        let id = format!("otp-digit-{position}");
                        let value = if digit != '\0' {
                            digit.to_string()
                        } else {
                            String::new()
                        };
                        let (handle_key_down, handle_paste) = create_handlers(position);
                        rsx! {
                            input {
                                key: id,
                                id,
                                r#type: "text",
                                maxlength: "1",
                                "data-bwignore": "true",
                                autocomplete: "off",
                                onkeydown: handle_key_down,
                                onpaste: handle_paste,
                                value,
                            }
                        }
                    })
            }
        }
    }
}

/// Apply updates to the code array, returns the position of the next digit to
/// be updated after the last updated digit
fn apply_code_update(current: &mut [char; 6], updates: &str, position: usize) -> usize {
    let mut index = position.saturating_sub(1);

    for char in updates.chars().take(current.len() - index) {
        if !char.is_numeric() {
            return index + 1;
        }

        current[index] = char;
        index += 1;
    }

    index + 1
}

fn focus_on_input(position: usize) {
    let next_input_id = format!("otp-digit-{position}");
    let _ = focus_element(&next_input_id);
}

pub fn code_is_filled_out(code: [char; 6]) -> bool {
    code.iter().all(|digit| digit.is_numeric())
}
