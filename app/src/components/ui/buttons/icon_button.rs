use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ButtonColor {
    Default,
    Danger,
}

impl ButtonColor {
    pub fn get_class_name(&self) -> &'static str {
        match self {
            ButtonColor::Default => "",
            ButtonColor::Danger => "icon-button--danger",
        }
    }
}

#[component]
pub fn IconButton(icon: Element, color: Option<ButtonColor>, on_press: Callback) -> Element {
    let class_name = color.unwrap_or(ButtonColor::Default).get_class_name();

    rsx! {
        button {
            class: format!("icon-button {}", class_name),
            onclick: move |_| on_press(()),
            {icon}
        }
    }
}
