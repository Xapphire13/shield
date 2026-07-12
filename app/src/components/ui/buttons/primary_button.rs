use dioxus::prelude::*;

use crate::components::buttons::base_button::BaseButton;

stylance::import_crate_style!(style, "src/components/ui/buttons/primary_button.module.css");

#[component]
pub fn PrimaryButton(
    class: Option<String>,
    on_press: Option<Callback>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        BaseButton {
            class: style::button,
            on_press,
            disabled,
            children,
        }
    }
}
