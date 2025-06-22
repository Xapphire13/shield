use dioxus::prelude::*;

#[component]
pub fn RowGroup(label: String, children: Element) -> Element {
    rsx! {
        div { class: "row-group",
            div { class: "row-group__label", {label} }
            div { class: "row-group__children", {children} }
        }
    }
}
