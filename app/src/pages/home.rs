use dioxus::prelude::*;

use crate::{
    components::{Camera, ConfirmationModal, ConfirmationModalType, GroupActions, RowGroup},
    hooks::{UseCamerasResult, use_cameras, use_update_recording_mode},
    utils::{get_camera_ids, get_camera_names_by_ids, group_cameras_by_tags},
};

#[component]
pub fn Home() -> Element {
    let UseCamerasResult { cameras, loading } = use_cameras();
    let update_recording_mode = use_update_recording_mode();
    let mut confirmation_modal_type = use_signal(|| ConfirmationModalType::None);
    let mut selected_camera_ids: Signal<Vec<String>> = use_signal(Vec::new);
    let (tag_groups, untagged_cameras) = group_cameras_by_tags(&cameras);

    let mut handle_toggle_record_on = move |camera_ids: Vec<String>| {
        selected_camera_ids.set(camera_ids);
        confirmation_modal_type.set(ConfirmationModalType::ConfirmToggleOn);
    };
    let mut handle_toggle_record_off = move |camera_ids: Vec<String>| {
        selected_camera_ids.set(camera_ids);
        confirmation_modal_type.set(ConfirmationModalType::ConfirmToggleOff);
    };
    let mut handle_close_confirmation_modal = move || {
        confirmation_modal_type.set(ConfirmationModalType::None);
    };

    let mut tags: Vec<String> = tag_groups.keys().cloned().collect();
    tags.sort();
    let tags = tags; // Remove mutability

    rsx! {
        div { class: "home-container",
            if loading {
                "Loading..."
            }

            {
                tags.iter()
                    .map(|tag| {
                        let cameras = tag_groups.get(tag).unwrap();
                        let camera_ids = get_camera_ids(cameras);
                        rsx! {
                            RowGroup {
                                label: tag,
                                actions: rsx! {
                                    GroupActions {
                                        on_toggle_record_on: {
                                            let camera_ids = camera_ids.clone();
                                            move || handle_toggle_record_on(camera_ids.clone())
                                        },
                                        on_toggle_record_off: {
                                            let camera_ids = camera_ids.clone();
                                            move || handle_toggle_record_off(camera_ids.clone())
                                        },
                                    }
                                },
                                {cameras.iter().map(|&camera| rsx! {
                                    Camera { camera: camera.clone() }
                                })}
                            }
                        }
                    })
            }

            if !untagged_cameras.is_empty() {
                RowGroup {
                    label: "Untagged",
                    actions: rsx! {
                        GroupActions {
                            on_toggle_record_on: {
                                let camera_ids = get_camera_ids(&untagged_cameras);
                                move || handle_toggle_record_on(camera_ids.clone())
                            },
                            on_toggle_record_off: {
                                let camera_ids = get_camera_ids(&untagged_cameras);
                                move || handle_toggle_record_off(camera_ids.clone())
                            },
                        }
                    },
                    {untagged_cameras.iter().map(|&camera| rsx! {
                        Camera { camera: camera.clone() }
                    })}
                }
            }

            match confirmation_modal_type() {
                ConfirmationModalType::ConfirmToggleOn => rsx! {
                    ConfirmationModal {
                        confirmation_type: ConfirmationModalType::ConfirmToggleOn,
                        on_close: handle_close_confirmation_modal,
                        on_confirm: move || {
                            update_recording_mode(
                                selected_camera_ids(),
                                shield_models::RecordingMode::Always,
                            );
                            handle_close_confirmation_modal();
                        },
                        camera_names: get_camera_names_by_ids(&cameras, &selected_camera_ids()),
                    }
                },
                ConfirmationModalType::ConfirmToggleOff => rsx! {
                    ConfirmationModal {
                        confirmation_type: ConfirmationModalType::ConfirmToggleOff,
                        on_close: handle_close_confirmation_modal,
                        on_confirm: move || {
                            update_recording_mode(
                                selected_camera_ids(),
                                shield_models::RecordingMode::Never,
                            );
                            handle_close_confirmation_modal();
                        },
                        camera_names: get_camera_names_by_ids(&cameras, &selected_camera_ids()),
                    }
                },
                ConfirmationModalType::None => rsx! {},
            }
        }
    }
}
