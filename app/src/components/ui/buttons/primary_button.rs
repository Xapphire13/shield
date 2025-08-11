use dioxus::prelude::*;

use crate::components::buttons::base_button::BaseButton;

#[component]
pub fn PrimaryButton(
    class: Option<String>,
    on_press: Option<Callback>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        BaseButton {
            class: "primary-button",
            on_press,
            disabled,
            children,
        }
    }
}
