use dioxus::prelude::*;
use shield_models::{
    DoorSwing, FieldOfView, Map, MapCamera, MapDoor, MapWall, Point, UpdateMapCameraRequest,
    UpdateMapDoorRequest, UpdateMapWallRequest, WallColor,
};

use crate::{api::MapApi, hooks::use_api_client::use_api_client};

/// A single, reversible edit to a [`Map`].
///
/// Each variant carries enough information to both **apply** the edit to a local
/// [`Map`] and to compute its **inverse** (for undo). The same representation is
/// also what drives the autosave: [`MapEdit::persist`] turns an edit into the
/// matching [`MapApi`] call.
///
/// `Move`/`Aim` capture both the `from` and `to` values so that undo/redo never
/// needs to read back from the server.
#[derive(Clone, PartialEq)]
pub enum MapEdit {
    /// A camera was placed on the map.
    AddCamera(MapCamera),
    /// A camera was removed from the map. Stores the full [`MapCamera`] so the
    /// inverse can re-add it verbatim.
    RemoveCamera(MapCamera),
    /// A camera's position changed.
    MoveCamera {
        camera_id: String,
        from: Point,
        to: Point,
    },
    /// A camera's field-of-view cone changed (direction / angle / range).
    UpdateFov {
        camera_id: String,
        from: FieldOfView,
        to: FieldOfView,
    },
    /// A wall was placed on the map.
    AddWall(MapWall),
    /// A wall was removed from the map. Stores the full [`MapWall`] so the
    /// inverse can re-add it verbatim.
    RemoveWall(MapWall),
    /// A wall's vertices changed.
    UpdateWallVertices {
        wall_id: String,
        from: Vec<Point>,
        to: Vec<Point>,
    },
    /// A wall's closed flag changed.
    UpdateWallClosed {
        wall_id: String,
        from: bool,
        to: bool,
    },
    /// A wall's display color changed.
    UpdateWallColor {
        wall_id: String,
        from: WallColor,
        to: WallColor,
    },
    /// A door was placed on the map.
    AddDoor(MapDoor),
    /// A door was removed from the map. Stores the full [`MapDoor`] so the
    /// inverse can re-add it verbatim.
    RemoveDoor(MapDoor),
    /// A door's start/end points changed (moved as a pair).
    MoveDoor {
        door_id: String,
        from: (Point, Point),
        to: (Point, Point),
    },
    /// A door's swing side was flipped.
    FlipDoorSwing {
        door_id: String,
        from: DoorSwing,
        to: DoorSwing,
    },
}

impl MapEdit {
    /// The edit that exactly reverses `self`. Replaying `self` then `self.invert()`
    /// leaves the map unchanged.
    fn invert(&self) -> MapEdit {
        match self {
            MapEdit::AddCamera(camera) => MapEdit::RemoveCamera(camera.clone()),
            MapEdit::RemoveCamera(camera) => MapEdit::AddCamera(camera.clone()),
            MapEdit::MoveCamera {
                camera_id,
                from,
                to,
            } => MapEdit::MoveCamera {
                camera_id: camera_id.clone(),
                from: to.clone(),
                to: from.clone(),
            },
            MapEdit::UpdateFov {
                camera_id,
                from,
                to,
            } => MapEdit::UpdateFov {
                camera_id: camera_id.clone(),
                from: to.clone(),
                to: from.clone(),
            },
            MapEdit::AddWall(wall) => MapEdit::RemoveWall(wall.clone()),
            MapEdit::RemoveWall(wall) => MapEdit::AddWall(wall.clone()),
            MapEdit::UpdateWallVertices { wall_id, from, to } => MapEdit::UpdateWallVertices {
                wall_id: wall_id.clone(),
                from: to.clone(),
                to: from.clone(),
            },
            MapEdit::UpdateWallClosed { wall_id, from, to } => MapEdit::UpdateWallClosed {
                wall_id: wall_id.clone(),
                from: *to,
                to: *from,
            },
            MapEdit::UpdateWallColor { wall_id, from, to } => MapEdit::UpdateWallColor {
                wall_id: wall_id.clone(),
                from: *to,
                to: *from,
            },
            MapEdit::AddDoor(door) => MapEdit::RemoveDoor(door.clone()),
            MapEdit::RemoveDoor(door) => MapEdit::AddDoor(door.clone()),
            MapEdit::MoveDoor { door_id, from, to } => MapEdit::MoveDoor {
                door_id: door_id.clone(),
                from: to.clone(),
                to: from.clone(),
            },
            MapEdit::FlipDoorSwing { door_id, from, to } => MapEdit::FlipDoorSwing {
                door_id: door_id.clone(),
                from: to.clone(),
                to: from.clone(),
            },
        }
    }

    /// Apply this edit to a local [`Map`] in place (optimistic update).
    fn apply(&self, map: &mut Map) {
        match self {
            MapEdit::AddCamera(camera) => map.cameras.push(camera.clone()),
            MapEdit::RemoveCamera(camera) => {
                map.cameras.retain(|c| c.camera_id != camera.camera_id);
            }
            MapEdit::MoveCamera { camera_id, to, .. } => {
                if let Some(camera) = map.cameras.iter_mut().find(|c| &c.camera_id == camera_id) {
                    camera.position = to.clone();
                }
            }
            MapEdit::UpdateFov { camera_id, to, .. } => {
                if let Some(camera) = map.cameras.iter_mut().find(|c| &c.camera_id == camera_id) {
                    camera.fov = to.clone();
                }
            }
            MapEdit::AddWall(wall) => map.walls.push(wall.clone()),
            MapEdit::RemoveWall(wall) => {
                map.walls.retain(|w| w.id != wall.id);
            }
            MapEdit::UpdateWallVertices { wall_id, to, .. } => {
                if let Some(wall) = map.walls.iter_mut().find(|w| &w.id == wall_id) {
                    wall.vertices = to.clone();
                }
            }
            MapEdit::UpdateWallClosed { wall_id, to, .. } => {
                if let Some(wall) = map.walls.iter_mut().find(|w| &w.id == wall_id) {
                    wall.closed = *to;
                }
            }
            MapEdit::UpdateWallColor { wall_id, to, .. } => {
                if let Some(wall) = map.walls.iter_mut().find(|w| &w.id == wall_id) {
                    wall.color = *to;
                }
            }
            MapEdit::AddDoor(door) => map.doors.push(door.clone()),
            MapEdit::RemoveDoor(door) => {
                map.doors.retain(|d| d.id != door.id);
            }
            MapEdit::MoveDoor { door_id, to, .. } => {
                if let Some(door) = map.doors.iter_mut().find(|d| &d.id == door_id) {
                    door.start = to.0.clone();
                    door.end = to.1.clone();
                }
            }
            MapEdit::FlipDoorSwing { door_id, to, .. } => {
                if let Some(door) = map.doors.iter_mut().find(|d| &d.id == door_id) {
                    door.swing = to.clone();
                }
            }
        }
    }

    /// Persist this edit via the [`MapApi`] (autosave). Returns the API result so
    /// callers can decide how to surface failures.
    async fn persist(
        &self,
        client: &crate::api::ApiClient,
        map_id: &str,
    ) -> Result<(), crate::types::ApiError> {
        match self {
            MapEdit::AddCamera(camera) => client.add_camera(map_id, camera.clone()).await,
            MapEdit::RemoveCamera(camera) => client.delete_camera(map_id, &camera.camera_id).await,
            MapEdit::MoveCamera { camera_id, to, .. } => {
                let update = UpdateMapCameraRequest {
                    position: Some(to.clone()),
                    fov: None,
                };
                client.update_camera(map_id, camera_id, update).await
            }
            MapEdit::UpdateFov { camera_id, to, .. } => {
                let update = UpdateMapCameraRequest {
                    position: None,
                    fov: Some(to.clone()),
                };
                client.update_camera(map_id, camera_id, update).await
            }
            MapEdit::AddWall(wall) => client.add_wall(map_id, wall.clone()).await,
            MapEdit::RemoveWall(wall) => client.delete_wall(map_id, &wall.id).await,
            MapEdit::UpdateWallVertices { wall_id, to, .. } => {
                let update = UpdateMapWallRequest {
                    vertices: Some(to.clone()),
                    closed: None,
                    color: None,
                };
                client.update_wall(map_id, wall_id, update).await
            }
            MapEdit::UpdateWallClosed { wall_id, to, .. } => {
                let update = UpdateMapWallRequest {
                    vertices: None,
                    closed: Some(*to),
                    color: None,
                };
                client.update_wall(map_id, wall_id, update).await
            }
            MapEdit::UpdateWallColor { wall_id, to, .. } => {
                let update = UpdateMapWallRequest {
                    vertices: None,
                    closed: None,
                    color: Some(*to),
                };
                client.update_wall(map_id, wall_id, update).await
            }
            MapEdit::AddDoor(door) => client.add_door(map_id, door.clone()).await,
            MapEdit::RemoveDoor(door) => client.delete_door(map_id, &door.id).await,
            MapEdit::MoveDoor { door_id, to, .. } => {
                let update = UpdateMapDoorRequest {
                    start: Some(to.0.clone()),
                    end: Some(to.1.clone()),
                    swing: None,
                };
                client.update_door(map_id, door_id, update).await
            }
            MapEdit::FlipDoorSwing { door_id, to, .. } => {
                let update = UpdateMapDoorRequest {
                    start: None,
                    end: None,
                    swing: Some(to.clone()),
                };
                client.update_door(map_id, door_id, update).await
            }
        }
    }
}

/// Result returned by [`use_map`]. Holds the current map state plus all of the
/// mutating actions and undo/redo controls.
#[derive(Clone)]
pub struct UseMapResult {
    /// The current (optimistically-updated) map, or `None` while loading / on
    /// load failure.
    pub map: Option<Map>,
    /// `true` until the initial [`MapApi::get_map`] resolves.
    pub loading: bool,
    /// Place a new camera on the map.
    pub place_camera: Callback<MapCamera>,
    /// Move an existing camera to a new position.
    pub move_camera: Callback<(String, Point)>,
    /// Re-aim an existing camera's field-of-view cone.
    pub aim_camera: Callback<(String, FieldOfView)>,
    /// Remove a camera from the map.
    pub remove_camera: Callback<String>,
    /// Place a new wall (or fence) on the map.
    pub place_wall: Callback<MapWall>,
    /// Replace an existing wall's vertices.
    pub update_wall_vertices: Callback<(String, Vec<Point>)>,
    /// Close an existing wall (add the implicit closing segment).
    pub close_wall: Callback<String>,
    /// Recolor an existing wall.
    pub recolor_wall: Callback<(String, WallColor)>,
    /// Remove a wall from the map.
    pub remove_wall: Callback<String>,
    /// Place a new door (or gate) on the map.
    pub place_door: Callback<MapDoor>,
    /// Move an existing door's start/end points.
    pub move_door: Callback<(String, Point, Point)>,
    /// Flip an existing door's swing side.
    pub flip_door_swing: Callback<String>,
    /// Remove a door from the map.
    pub remove_door: Callback<String>,
    /// Undo the most recent edit.
    pub undo: Callback<()>,
    /// Redo the most recently undone edit.
    pub redo: Callback<()>,
    /// Whether there is an edit available to undo.
    pub can_undo: bool,
    /// Whether there is an edit available to redo.
    pub can_redo: bool,
}

/// Hook that loads a single map and exposes an editable, optimistic view of it
/// with autosave and an undo/redo stack.
///
/// # State model
///
/// - `map` mirrors the server map and is mutated optimistically on every action.
/// - `undo_stack` holds applied [`MapEdit`]s in chronological order; `redo_stack`
///   holds edits that were undone and can be reapplied.
/// - Performing any *new* edit clears the redo stack (the usual undo/redo
///   invariant).
///
/// Each action: (1) builds the [`MapEdit`], (2) applies it to local state, (3)
/// pushes it onto the undo stack / clears redo, and (4) fires the API call in a
/// background task (autosave). `undo`/`redo` move an edit between the two stacks,
/// applying the inverse / original edit locally and persisting it.
pub fn use_map(map_id: String) -> UseMapResult {
    let client = use_api_client();

    // Optimistic local copy of the map. Seeded by the resource below.
    let mut map = use_signal(|| None::<Map>);
    let mut undo_stack = use_signal(Vec::<MapEdit>::new);
    let mut redo_stack = use_signal(Vec::<MapEdit>::new);

    let map_resource = use_resource({
        let client = client.clone();
        let map_id = map_id.clone();
        move || {
            let client = client.clone();
            let map_id = map_id.clone();
            async move { client.get_map(&map_id).await.ok() }
        }
    });

    // Seed local state once the resource resolves (or re-resolves).
    use_effect(move || {
        if let Some(loaded) = map_resource() {
            map.set(loaded);
        }
    });

    // Commit a brand-new edit: apply locally, record on the undo stack, clear
    // the redo stack, and autosave.
    let commit = {
        let client = client.clone();
        let map_id = map_id.clone();
        move |edit: MapEdit| {
            let Some(mut current) = map() else {
                return;
            };
            edit.apply(&mut current);
            map.set(Some(current));

            undo_stack.write().push(edit.clone());
            redo_stack.write().clear();

            let client = client.clone();
            let map_id = map_id.clone();
            spawn(async move {
                let _ = edit.persist(&client, &map_id).await;
            });
        }
    };

    let place_camera = use_callback({
        let mut commit = commit.clone();
        move |camera: MapCamera| commit(MapEdit::AddCamera(camera))
    });

    let move_camera = use_callback({
        let mut commit = commit.clone();
        move |(camera_id, to): (String, Point)| {
            let Some(from) = map()
                .and_then(|m| m.cameras.into_iter().find(|c| c.camera_id == camera_id))
                .map(|c| c.position)
            else {
                return;
            };

            if from == to {
                return;
            }

            commit(MapEdit::MoveCamera {
                camera_id,
                from,
                to,
            });
        }
    });

    let aim_camera = use_callback({
        let mut commit = commit.clone();
        move |(camera_id, to): (String, FieldOfView)| {
            let Some(from) = map()
                .and_then(|m| m.cameras.into_iter().find(|c| c.camera_id == camera_id))
                .map(|c| c.fov)
            else {
                return;
            };

            if from == to {
                return;
            }

            commit(MapEdit::UpdateFov {
                camera_id,
                from,
                to,
            });
        }
    });

    let remove_camera = use_callback({
        let mut commit = commit.clone();
        move |camera_id: String| {
            let Some(camera) =
                map().and_then(|m| m.cameras.into_iter().find(|c| c.camera_id == camera_id))
            else {
                return;
            };

            commit(MapEdit::RemoveCamera(camera))
        }
    });

    let place_wall = use_callback({
        let mut commit = commit.clone();
        move |wall: MapWall| commit(MapEdit::AddWall(wall))
    });

    let update_wall_vertices = use_callback({
        let mut commit = commit.clone();
        move |(wall_id, to): (String, Vec<Point>)| {
            let Some(from) = map()
                .and_then(|m| m.walls.into_iter().find(|w| w.id == wall_id))
                .map(|w| w.vertices)
            else {
                return;
            };

            if from == to {
                return;
            }

            commit(MapEdit::UpdateWallVertices { wall_id, from, to });
        }
    });

    let close_wall = use_callback({
        let mut commit = commit.clone();
        move |wall_id: String| {
            let Some(closed) = map()
                .and_then(|m| m.walls.into_iter().find(|w| w.id == wall_id))
                .map(|w| w.closed)
            else {
                return;
            };

            if closed {
                return;
            }

            commit(MapEdit::UpdateWallClosed {
                wall_id,
                from: false,
                to: true,
            });
        }
    });

    let recolor_wall = use_callback({
        let mut commit = commit.clone();
        move |(wall_id, to): (String, WallColor)| {
            let Some(from) = map()
                .and_then(|m| m.walls.into_iter().find(|w| w.id == wall_id))
                .map(|w| w.color)
            else {
                return;
            };

            if from == to {
                return;
            }

            commit(MapEdit::UpdateWallColor { wall_id, from, to });
        }
    });

    let remove_wall = use_callback({
        let mut commit = commit.clone();
        move |wall_id: String| {
            let Some(wall) = map().and_then(|m| m.walls.into_iter().find(|w| w.id == wall_id))
            else {
                return;
            };

            commit(MapEdit::RemoveWall(wall))
        }
    });

    let place_door = use_callback({
        let mut commit = commit.clone();
        move |door: MapDoor| commit(MapEdit::AddDoor(door))
    });

    let move_door = use_callback({
        let mut commit = commit.clone();
        move |(door_id, new_start, new_end): (String, Point, Point)| {
            let Some(from) = map()
                .and_then(|m| m.doors.into_iter().find(|d| d.id == door_id))
                .map(|d| (d.start, d.end))
            else {
                return;
            };

            let to = (new_start, new_end);
            if from == to {
                return;
            }

            commit(MapEdit::MoveDoor { door_id, from, to });
        }
    });

    let flip_door_swing = use_callback({
        let mut commit = commit.clone();
        move |door_id: String| {
            let Some(from) = map()
                .and_then(|m| m.doors.into_iter().find(|d| d.id == door_id))
                .map(|d| d.swing)
            else {
                return;
            };

            let to = match from {
                DoorSwing::Left => DoorSwing::Right,
                DoorSwing::Right => DoorSwing::Left,
            };

            commit(MapEdit::FlipDoorSwing { door_id, from, to });
        }
    });

    let remove_door = use_callback({
        let mut commit = commit.clone();
        move |door_id: String| {
            let Some(door) = map().and_then(|m| m.doors.into_iter().find(|d| d.id == door_id))
            else {
                return;
            };

            commit(MapEdit::RemoveDoor(door))
        }
    });

    let undo = use_callback({
        let client = client.clone();
        let map_id = map_id.clone();
        move |()| {
            let Some(edit) = undo_stack.write().pop() else {
                return;
            };
            let inverse = edit.invert();

            if let Some(mut current) = map() {
                inverse.apply(&mut current);
                map.set(Some(current));
            }

            redo_stack.write().push(edit);

            let client = client.clone();
            let map_id = map_id.clone();
            spawn(async move {
                let _ = inverse.persist(&client, &map_id).await;
            });
        }
    });

    let redo = use_callback({
        let client = client.clone();
        let map_id = map_id.clone();
        move |()| {
            let Some(edit) = redo_stack.write().pop() else {
                return;
            };

            if let Some(mut current) = map() {
                edit.apply(&mut current);
                map.set(Some(current));
            }

            undo_stack.write().push(edit.clone());

            let client = client.clone();
            let map_id = map_id.clone();
            spawn(async move {
                let _ = edit.persist(&client, &map_id).await;
            });
        }
    });

    UseMapResult {
        map: map(),
        loading: !map_resource.finished(),
        place_camera,
        move_camera,
        aim_camera,
        remove_camera,
        place_wall,
        update_wall_vertices,
        close_wall,
        recolor_wall,
        remove_wall,
        place_door,
        move_door,
        flip_door_swing,
        remove_door,
        undo,
        redo,
        can_undo: !undo_stack.read().is_empty(),
        can_redo: !redo_stack.read().is_empty(),
    }
}
