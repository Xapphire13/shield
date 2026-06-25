use dioxus::prelude::*;

#[component]
pub fn DisclosureRow(
    header: String,
    sub_header: String,
    after: Element,
    #[props(default)] selected: bool,
    on_click: Callback<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: "disclosure-row",
            "data-selected": selected,
            onclick: move |event| on_click.call(event),

            div { class: "disclosure-row__content",
                div {
                    div { class: "disclosure-row__header", {header} }
                    div { class: "disclosure-row__sub-header", {sub_header} }
                }
            }

            div { {after} }
        }
    }
}
