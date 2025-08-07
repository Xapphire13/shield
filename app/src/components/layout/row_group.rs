use dioxus::prelude::*;

#[component]
pub fn RowGroup(label: String, actions: Element, children: Element) -> Element {
    rsx! {
        div { class: "row-group",
            div { class: "row-group__header",
                div { class: "row-group__label", {label} }

                {actions}
            }
            div { class: "row-group__children", {children} }
        }
    }
}
