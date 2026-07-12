use dioxus::prelude::*;

stylance::import_crate_style!(style, "src/components/ui/selectable_row.module.css");

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
            class: style::container,
            "data-selected": selected,
            onclick: move |event| on_click.call(event),

            div { class: style::content,
                div {
                    div { class: style::header, {header} }
                    div { class: style::sub_header, {sub_header} }
                }
            }

            div { {after} }
        }
    }
}
