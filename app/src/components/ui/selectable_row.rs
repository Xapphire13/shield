use dioxus::prelude::*;

#[component]
pub fn SelectableRow(
    header: String,
    sub_header: String,
    after: Element,
    #[props(default)] selected: bool,
    on_click: Callback<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: "selectable-row",
            "data-selected": selected,
            onclick: move |event| on_click.call(event),

            div { class: "selectable-row__content",
                div {
                    div { class: "selectable-row__header", {header} }
                    div { class: "selectable-row__sub-header", {sub_header} }
                }
            }

            div { {after} }
        }
    }
}
