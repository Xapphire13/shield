use dioxus::prelude::*;

#[component]
pub fn SecondaryButton(on_press: Option<Callback>, children: Element) -> Element {
    rsx! {
        button {
            class: "secondary-button",
            onclick: move |_| {
                if let Some(on_press) = on_press {
                    on_press(())
                }
            },
            {children}
        }
    }
}
