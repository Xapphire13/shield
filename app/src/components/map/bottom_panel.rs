//! The edit-mode bottom chrome: a contextual inspector for the selected
//! camera / wall / door, or the tool strip when nothing is selected.

use dioxus::prelude::*;
use shield_models::{FieldOfView, MapCamera, MapDoor, MapWall, WallColor};

use crate::components::map::camera_inspector::CameraInspector;
use crate::components::map::door_inspector::DoorInspector;
use crate::components::map::edit_toolbar::EditToolbar;
use crate::components::map::interaction::{DragPreview, Selection, Tool};
use crate::components::map::wall_inspector::WallInspector;

/// Bottom chrome shown in edit mode. The inspector takes precedence over the
/// tool strip when a camera, wall, or door is selected (camera first, then
/// wall, then door). Both sit above the global navigation toolbar.
///
/// The selected elements arrive with any active drag preview already applied
/// (the host derives them from its display data), so the inspector's values
/// track in-progress gestures live.
#[component]
pub fn BottomPanel(
    /// The selected camera, with the active preview applied.
    selected_camera: Option<MapCamera>,
    /// Display name for the selected camera; `None` marks an orphaned
    /// reference (underlying camera deleted).
    selected_camera_name: Option<String>,
    /// The selected wall, with the active preview applied.
    selected_wall: Option<MapWall>,
    /// The selected door, with the active preview applied.
    selected_door: Option<MapDoor>,
    tool: Signal<Tool>,
    picker_open: Signal<bool>,
    drag_preview: Signal<DragPreview>,
    selection: Signal<Option<Selection>>,
    aim_camera: Callback<(String, FieldOfView)>,
    remove_camera: Callback<String>,
    place_wall: Callback<MapWall>,
    close_wall: Callback<String>,
    recolor_wall: Callback<(String, WallColor)>,
    remove_wall: Callback<String>,
    flip_door_swing: Callback<String>,
    remove_door: Callback<String>,
) -> Element {
    rsx! {
        if let Some(camera) = selected_camera {
            CameraInspector {
                name: selected_camera_name,
                fov: camera.fov.clone(),
                on_preview_fov: {
                    // Live, uncommitted preview: drive the same drag
                    // preview the on-canvas handles use so the cone
                    // (and the inspector's own `fov` prop) update in
                    // real time without persisting or touching undo.
                    let id = camera.camera_id.clone();
                    move |fov| {
                        drag_preview
                            .set(DragPreview::Fov {
                                camera_id: id.clone(),
                                fov,
                            });
                    }
                },
                on_change_fov: {
                    // Release: commit one edit and drop the preview.
                    let id = camera.camera_id.clone();
                    move |fov| {
                        aim_camera((id.clone(), fov));
                        drag_preview.set(DragPreview::None);
                    }
                },
                on_delete: {
                    let id = camera.camera_id.clone();
                    move |_| {
                        remove_camera(id.clone());
                        selection.set(None);
                    }
                },
            }
        } else if let Some(wall) = selected_wall {
            WallInspector {
                wall: wall.clone(),
                on_close_loop: {
                    let id = wall.id.clone();
                    move |_| close_wall(id.clone())
                },
                on_recolor: {
                    let id = wall.id.clone();
                    move |color| recolor_wall((id.clone(), color))
                },
                on_delete: {
                    let id = wall.id.clone();
                    move |_| {
                        remove_wall(id.clone());
                        selection.set(None);
                    }
                },
            }
        } else if let Some(door) = selected_door {
            DoorInspector {
                door: door.clone(),
                on_flip_swing: {
                    let id = door.id.clone();
                    move |_| flip_door_swing(id.clone())
                },
                on_delete: {
                    let id = door.id.clone();
                    move |_| {
                        remove_door(id.clone());
                        selection.set(None);
                    }
                },
            }
        } else {
            EditToolbar {
                active_tool: tool.read().clone(),
                camera_picker_open: *picker_open.read(),
                on_select: move |_| {
                    tool.set(Tool::Select);
                    picker_open.set(false);
                },
                on_add_camera: move |_| {
                    tool.set(Tool::Select);
                    picker_open.set(true);
                },
                on_draw_wall: move |_| {
                    tool.set(Tool::DrawWall { vertices: Vec::new() });
                    picker_open.set(false);
                },
                on_finish_wall: move |_| {
                    if let Tool::DrawWall { vertices } = tool.read().clone()
                        && vertices.len() >= 2
                    {
                        place_wall(MapWall {
                            id: uuid::Uuid::new_v4().to_string(),
                            vertices,
                            closed: false,
                            color: WallColor::default(),
                        });
                    }
                    tool.set(Tool::Select);
                },
                on_place_door: move |_| {
                    tool.set(Tool::PlaceDoor { start: None });
                    picker_open.set(false);
                },
            }
        }
    }
}
