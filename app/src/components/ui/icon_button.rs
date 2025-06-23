use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ButtonColor {
    Default,
    Danger,
}

impl ButtonColor {
    pub fn get_css_colors(&self) -> (String, String) {
        match self {
            ButtonColor::Default => ("#535e7a".to_owned(), "#616e8e".to_owned()),
            ButtonColor::Danger => ("#fa486b".to_owned(), "#fb6180".to_owned()),
        }
    }
}

#[component]
pub fn IconButton(icon: Element, color: Option<ButtonColor>) -> Element {
    let (background_color, border_color) = color.unwrap_or(ButtonColor::Default).get_css_colors();

    rsx! {
        button {
            class: "icon-button",
            style: format!("background: {background_color}; border-color: {border_color};"),
            {icon}
        }
    }
}
