use dioxus::prelude::*;

use crate::{
    ConfirmationModalType,
    components::ui::{Modal, ModalBody, ModalFooter, ModalHeader, PrimaryButton, SecondaryButton},
};

#[component]
pub fn ConfirmationModal(
    confirmation_type: ConfirmationModalType,
    on_close: Callback,
    camera_names: Vec<String>,
) -> Element {
    rsx! {
        Modal { on_close,
            ModalHeader { "Are you sure?" }
            ModalBody {
                match confirmation_type {
                    ConfirmationModalType::ConfirmToggleOn => {
                        "This will enable recording on the following cameras:"
                    }
                    ConfirmationModalType::ConfirmToggleOff => {
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
                PrimaryButton { "Confirm" }
            }
        }
    }
}
