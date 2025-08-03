use dioxus::prelude::*;
use std::collections::HashMap;

use crate::{
    components::{Camera, ConfirmationModal, ConfirmationModalType, GroupActions, ui::RowGroup},
    use_api_client::use_api_client,
    use_update_recording_mode::use_update_recording_mode,
};

#[component]
pub fn Home() -> Element {
    let client = use_api_client();
    let cameras = use_resource(move || async move {
        client
            .as_ref()
            .unwrap()
            .get_cameras()
            .await
            .unwrap_or(Vec::new())
    });
    let update_recording_mode = use_update_recording_mode();
    let mut confirmation_modal_type = use_signal(|| ConfirmationModalType::None);
    let mut selected_camera_ids: Signal<Vec<String>> = use_signal(Vec::new);
    let cameras = cameras.cloned().unwrap_or_else(Vec::new);
    let mut tag_groups: HashMap<String, Vec<&shield_models::Camera>> = HashMap::new();
    let mut untagged_cameras = vec![];

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

    for camera in cameras.iter() {
        for tag in &camera.tags {
            tag_groups
                .entry(tag.to_owned())
                .and_modify(|group| {
                    group.push(camera);
                })
                .or_insert_with(|| vec![camera]);
        }

        if camera.tags.is_empty() {
            untagged_cameras.push(camera);
        }
    }
    let tags = {
        let mut tags: Vec<String> = tag_groups.keys().cloned().collect();
        tags.sort();
        tags
    };

    rsx! {
        div { class: "home-container",
            {
                tags.iter()
                    .map(|tag| {
                        let cameras = tag_groups.get(tag).unwrap();
                        let camera_ids: Vec<String> = cameras
                            .iter()
                            .map(|camera| camera.id.clone())
                            .collect();
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
                                let camera_ids = untagged_cameras
                                    .iter()
                                    .map(|&camera| camera.id.clone())
                                    .collect::<Vec<_>>();
                                move || handle_toggle_record_on(camera_ids.clone())
                            },
                            on_toggle_record_off: {
                                let camera_ids = untagged_cameras
                                    .iter()
                                    .map(|&camera| camera.id.clone())
                                    .collect::<Vec<_>>();
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
                        camera_names: selected_camera_ids()
                            .iter()
                            .flat_map(|id| {
                                cameras
                                    .iter()
                                    .find_map(|camera| {
                                        if &camera.id == id { Some(camera.name.clone()) } else { None }
                                    })
                            })
                            .collect(),
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
                        camera_names: selected_camera_ids()
                            .iter()
                            .flat_map(|id| {
                                cameras
                                    .iter()
                                    .find_map(|camera| {
                                        if &camera.id == id { Some(camera.name.clone()) } else { None }
                                    })
                            })
                            .collect(),
                    }
                },
                ConfirmationModalType::None => rsx! {},
            }
        }
    }
}
