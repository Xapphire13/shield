use dioxus::prelude::*;

use crate::components::ui::{
    Modal, ModalBody, ModalFooter, ModalHeader, PrimaryButton, SecondaryButton,
};

#[derive(Clone, PartialEq)]
pub enum ConfirmationModalType {
    ConfirmToggleOn(Vec<String>),
    ConfirmToggleOff(Vec<String>),
    None,
}

#[component]
pub fn ConfirmationModal(
    confirmation_type: ConfirmationModalType,
    on_close: Callback,
    on_confirm: Callback,
    camera_names: Vec<String>,
) -> Element {
    rsx! {
        Modal { on_close,
            ModalHeader { "Are you sure?" }
            ModalBody {
                match confirmation_type {
                    ConfirmationModalType::ConfirmToggleOn(_) => {
                        "This will enable recording on the following cameras:"
                    }
                    ConfirmationModalType::ConfirmToggleOff(_) => {
                        "This will disabled recording on the following cameras:"
                    }
                    _ => "",
                }

                ul {
                    {
                        camera_names
                            .iter()
                            .map(|name| {
                                rsx! {
                                    li { {name.clone()} }
                                }
                            })
                    }
                }
            }
            ModalFooter {
                SecondaryButton { on_press: on_close, "Cancel" }
                PrimaryButton { on_press: on_confirm, "Confirm" }
            }
        }
    }
}
