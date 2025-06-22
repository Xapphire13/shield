use dioxus::prelude::*;
use dioxus_feather_icons::icon;

#[component]
pub fn DisclosureRow(header: String, sub_header: String, after: Element) -> Element {
    rsx! {
        div { class: "disclosure-row",

            div { class: "disclosure-row__content",
                div {
                    div { class: "disclosure-row__header", {header} }
                    div { class: "disclosure-row__sub-header", {sub_header} }
                }
            }

            div { {after} }

            div { {icon!(chevron_right)} }
        }
    }
}
