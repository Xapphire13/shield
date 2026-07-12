use dioxus::prelude::*;

stylance::import_crate_style!(style, "src/components/ui/buttons/base_button.module.css");

#[component]
pub fn BaseButton(
    class: Option<String>,
    on_press: Option<Callback>,
    disabled: Option<bool>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: format!("{} {}", style::button, class.unwrap_or(String::new())),
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
