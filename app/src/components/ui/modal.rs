use dioxus::prelude::*;

stylance::import_crate_style!(style, "src/components/ui/modal.module.css");

#[component]
pub fn Modal(children: Element, on_close: Callback) -> Element {
    rsx! {
        div { class: style::container, onclick: move |_| on_close(()),
            div {
                class: style::modal,
                onclick: |ev| {
                    ev.stop_propagation();
                },
                {children}
            }
        }
    }
}

#[component]
pub fn ModalHeader(children: Element) -> Element {
    rsx! {
        div { class: style::header, {children} }
    }
}

#[component]
pub fn ModalBody(children: Element) -> Element {
    rsx! {
        div { class: style::body, {children} }
    }
}

#[component]
pub fn ModalFooter(children: Element) -> Element {
    rsx! {
        div { class: style::footer, {children} }
    }
}
