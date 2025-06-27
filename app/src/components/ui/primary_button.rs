use dioxus::prelude::*;

#[component]
pub fn PrimaryButton(on_press: Option<Callback>, children: Element) -> Element {
    rsx! {
        button {
            class: "primary-button",
            onclick: move |_| {
                if let Some(on_press) = on_press {
                    on_press(())
                }
            },
            {children}
        }
    }
}
