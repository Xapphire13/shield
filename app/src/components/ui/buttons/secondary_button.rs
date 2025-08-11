use dioxus::prelude::*;

use crate::components::buttons::base_button::BaseButton;

#[component]
pub fn SecondaryButton(
    class: Option<String>,
    on_press: Option<Callback>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        BaseButton {
            class: "secondary-button",
            on_press,
            disabled,
            children,
        }
    }
}
