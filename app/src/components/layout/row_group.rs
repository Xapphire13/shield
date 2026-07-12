use dioxus::prelude::*;

stylance::import_crate_style!(style, "src/components/layout/row_group.module.css");

#[component]
pub fn RowGroup(label: String, actions: Element, children: Element) -> Element {
    rsx! {
        div { class: style::container,
            div { class: style::header,
                div { class: style::label, {label} }

                {actions}
            }
            div { class: style::children, {children} }
        }
    }
}
