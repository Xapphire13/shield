use dioxus::prelude::*;

#[component]
pub fn Modal(children: Element, on_close: Callback) -> Element {
    rsx! {
        div { class: "modal-container", onclick: move |_| on_close(()),
            div {
                class: "modal",
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
        div { class: "modal-header", {children} }
    }
}

#[component]
pub fn ModalBody(children: Element) -> Element {
    rsx! {
        div { class: "modal-body", {children} }
    }
}

#[component]
pub fn ModalFooter(children: Element) -> Element {
    rsx! {
        div { class: "modal-footer", {children} }
    }
}
