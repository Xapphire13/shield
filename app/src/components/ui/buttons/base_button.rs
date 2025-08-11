use dioxus::prelude::*;

#[component]
pub fn BaseButton(
    class: Option<String>,
    on_press: Option<Callback>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: format!("button {}", class.unwrap_or(String::new())),
            onclick: move |_| {
                if let Some(on_press) = on_press {
                    on_press(())
                }
            },
            disabled,
            {children}
        }
    }
}
