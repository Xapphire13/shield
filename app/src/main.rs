use std::collections::HashMap;

use dioxus::prelude::*;

use crate::components::{Camera, ConfirmationModal, GroupActions, ui::RowGroup};

mod components;
mod use_update_recording_mode;

use use_update_recording_mode::use_update_recording_mode;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, PartialEq)]
enum ConfirmationModalType {
    ConfirmToggleOn,
    ConfirmToggleOff,
    None,
}

fn get_api_url(path: &str) -> String {
    let hostname = web_sys::window().unwrap().location().hostname().unwrap();

    format!("http://{hostname}:3000{path}")
}

#[component]
fn App() -> Element {
    let cameras = use_resource(|| async move {
        let url = get_api_url("/cameras");

        reqwest::get(url)
            .await
            .unwrap()
            .json::<Vec<shield_models::Camera>>()
            .await
            .unwrap()
    });
    let update_recording_mode = use_update_recording_mode();
    let mut confirmation_modal_type = use_signal(|| ConfirmationModalType::None);
    let mut selected_camera_ids: Signal<Vec<String>> = use_signal(|| vec![]);
    let cameras = cameras.cloned().unwrap_or_else(|| vec![]);
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
        document::Link { rel: "stylesheet", href: MAIN_CSS }

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

        {dioxus_feather_icons::sprite!()}
    }
}
