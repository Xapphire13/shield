use dioxus::prelude::*;

#[component]
pub fn PrimaryButton(on_press: Option<Callback>, children: Element, id: Option<String>) -> Element {
    rsx! {
        button {
            class: "primary-button",
            id,
            onclick: move |_| {
                if let Some(on_press) = on_press {
                    on_press(())
                }
            },
            {children}
        }
    }
}
