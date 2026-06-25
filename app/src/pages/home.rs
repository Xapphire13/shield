use dioxus::prelude::*;

use crate::{
    components::{
        Camera, CameraActions, ConfirmationModal, ConfirmationModalType, GroupActions, RowGroup,
    },
    hooks::{UseCamerasResult, use_cameras, use_update_recording_mode},
    utils::{get_camera_ids, get_camera_names_by_ids, group_cameras_by_tags},
};

/// Remove cameras that were just acted on from the current selection.
fn deselect(selection: &mut Signal<Vec<String>>, camera_ids: &[String]) {
    selection.with_mut(|ids| ids.retain(|id| !camera_ids.contains(id)));
}

#[component]
pub fn Home() -> Element {
    let UseCamerasResult { cameras, loading } = use_cameras();
    let update_recording_mode = use_update_recording_mode();
    let mut confirmation_modal_type = use_signal(|| ConfirmationModalType::None);
    let mut selection: Signal<Vec<String>> = use_signal(Vec::new);
    let (tag_groups, untagged_cameras) = group_cameras_by_tags(&cameras);

    // Drop any selected ids that are no longer present in the current camera set.
    let all_camera_ids: Vec<String> = cameras.iter().map(|camera| camera.id.clone()).collect();
    use_effect(use_reactive((&all_camera_ids,), move |(all_camera_ids,)| {
        if selection
            .peek()
            .iter()
            .any(|id| !all_camera_ids.contains(id))
        {
            selection.with_mut(|ids| ids.retain(|id| all_camera_ids.contains(id)));
        }
    }));

    let handle_select_camera = move |id: String| {
        selection.with_mut(
            |ids| match ids.iter().position(|existing| existing == &id) {
                Some(index) => {
                    ids.remove(index);
                }
                None => ids.push(id),
            },
        );
    };

    let mut handle_toggle_record_on = move |camera_ids: Vec<String>| {
        confirmation_modal_type.set(ConfirmationModalType::ConfirmToggleOn(camera_ids));
    };
    let mut handle_toggle_record_off = move |camera_ids: Vec<String>| {
        confirmation_modal_type.set(ConfirmationModalType::ConfirmToggleOff(camera_ids));
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
                                    Camera {
                                        camera: camera.clone(),
                                        selected: selection().contains(&camera.id),
                                        on_select: handle_select_camera,
                                    }
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
                        Camera {
                            camera: camera.clone(),
                            selected: selection().contains(&camera.id),
                            on_select: handle_select_camera,
                        }
                    })}
                }
            }

            match confirmation_modal_type() {
                ConfirmationModalType::ConfirmToggleOn(camera_ids) => rsx! {
                    ConfirmationModal {
                        confirmation_type: ConfirmationModalType::ConfirmToggleOn(camera_ids.clone()),
                        on_close: handle_close_confirmation_modal,
                        on_confirm: {
                            let camera_ids = camera_ids.clone();
                            move || {
                                update_recording_mode(
                                    camera_ids.clone(),
                                    shield_models::RecordingMode::Always,
                                );
                                deselect(&mut selection, &camera_ids);
                                handle_close_confirmation_modal();
                            }
                        },
                        camera_names: get_camera_names_by_ids(&cameras, &camera_ids),
                    }
                },
                ConfirmationModalType::ConfirmToggleOff(camera_ids) => rsx! {
                    ConfirmationModal {
                        confirmation_type: ConfirmationModalType::ConfirmToggleOff(camera_ids.clone()),
                        on_close: handle_close_confirmation_modal,
                        on_confirm: {
                            let camera_ids = camera_ids.clone();
                            move || {
                                update_recording_mode(
                                    camera_ids.clone(),
                                    shield_models::RecordingMode::Never,
                                );
                                deselect(&mut selection, &camera_ids);
                                handle_close_confirmation_modal();
                            }
                        },
                        camera_names: get_camera_names_by_ids(&cameras, &camera_ids),
                    }
                },
                ConfirmationModalType::None => rsx! {},
            }

            CameraActions {
                visible: !selection().is_empty(),
                on_toggle_record_on: move || handle_toggle_record_on(selection()),
                on_toggle_record_off: move || handle_toggle_record_off(selection()),
                on_dismiss: move || selection.set(Vec::new()),
            }
        }
    }
}
