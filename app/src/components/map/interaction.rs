//! Interaction state for the map canvas: the active gesture, the local
//! uncommitted drag preview, the armed editing tool, and the current
//! selection — plus the pure state transitions over them (Escape's
//! back-out cascade, applying a preview to the stored map data).

use shield_models::{FieldOfView, MapCamera, MapDoor, MapWall, Point};

use crate::components::map::map_door::Endpoint;

/// Active gesture being tracked across pointer/touch events.
///
/// Pan starts on empty canvas; the camera-manipulation gestures start on a
/// marker / handle (which stops propagation so the canvas pan handler does not
/// also fire — this is the target-based disambiguation). Manipulation gestures
/// preview locally and commit exactly one edit on release.
#[derive(Clone, PartialEq)]
pub enum Gesture {
    None,
    /// One-pointer pan; stores the last screen position seen.
    Pan {
        last_x: f64,
        last_y: f64,
    },
    /// Two-finger pinch; stores the last finger distance (the midpoint is
    /// recomputed each move and used as the zoom anchor).
    Pinch {
        last_distance: f64,
    },
    /// Dragging a selected camera's body. Tracks the last screen position so the
    /// per-move delta can be converted to world cm.
    MoveCamera {
        camera_id: String,
        last_x: f64,
        last_y: f64,
    },
    /// Dragging the aim handle (rotates the cone toward the pointer).
    AimCamera {
        camera_id: String,
    },
    /// Dragging the range handle (lengthens / shortens the cone).
    RangeCamera {
        camera_id: String,
    },
    /// Dragging a single vertex of a selected wall. Tracks the last screen
    /// position so the per-move delta can be converted to world cm, same
    /// shape as `MoveCamera`.
    MoveWallVertex {
        wall_id: String,
        vertex_index: usize,
        last_x: f64,
        last_y: f64,
    },
    /// Dragging a single endpoint of a selected door. Tracks the last screen
    /// position so the per-move delta can be converted to world cm, same
    /// shape as `MoveWallVertex`.
    MoveDoorEndpoint {
        door_id: String,
        which: Endpoint,
        last_x: f64,
        last_y: f64,
    },
}

impl Gesture {
    /// Stable label for the active gesture, surfaced as a `data-gesture`
    /// attribute on the canvas so the cursor stays consistent while dragging
    /// even as the pointer crosses child elements.
    pub fn label(&self) -> &'static str {
        match self {
            Gesture::None => "none",
            Gesture::Pan { .. } => "pan",
            Gesture::Pinch { .. } => "pinch",
            Gesture::MoveCamera { .. } => "move",
            Gesture::AimCamera { .. } => "aim",
            Gesture::RangeCamera { .. } => "range",
            Gesture::MoveWallVertex { .. } => "move-vertex",
            Gesture::MoveDoorEndpoint { .. } => "move-endpoint",
        }
    }
}

/// A local, uncommitted preview of an in-progress manipulation. The canvas
/// renders from this instead of the stored map while a gesture is active so the
/// user sees live feedback; the matching edit is committed once on release.
#[derive(Clone, PartialEq)]
pub enum DragPreview {
    None,
    Position {
        camera_id: String,
        position: Point,
    },
    Fov {
        camera_id: String,
        fov: FieldOfView,
    },
    WallVertex {
        wall_id: String,
        vertex_index: usize,
        position: Point,
    },
    DoorEndpoint {
        door_id: String,
        which: Endpoint,
        position: Point,
    },
}

impl DragPreview {
    /// The previewed position for `camera_id`, if a position preview is active.
    pub fn position_for(&self, camera_id: &str) -> Option<Point> {
        match self {
            DragPreview::Position {
                camera_id: id,
                position,
            } if id == camera_id => Some(position.clone()),
            _ => None,
        }
    }

    /// The previewed FOV for `camera_id`, if a FOV preview is active.
    pub fn fov_for(&self, camera_id: &str) -> Option<FieldOfView> {
        match self {
            DragPreview::Fov { camera_id: id, fov } if id == camera_id => Some(fov.clone()),
            _ => None,
        }
    }

    /// The previewed position for vertex `vertex_index` of `wall_id`, if a
    /// matching wall-vertex preview is active.
    pub fn wall_vertex_for(&self, wall_id: &str, vertex_index: usize) -> Option<Point> {
        match self {
            DragPreview::WallVertex {
                wall_id: id,
                vertex_index: idx,
                position,
            } if id == wall_id && *idx == vertex_index => Some(position.clone()),
            _ => None,
        }
    }

    /// The previewed position for endpoint `which` of `door_id`, if a
    /// matching door-endpoint preview is active.
    pub fn door_endpoint_for(&self, door_id: &str, which: Endpoint) -> Option<Point> {
        match self {
            DragPreview::DoorEndpoint {
                door_id: id,
                which: w,
                position,
            } if id == door_id && *w == which => Some(position.clone()),
            _ => None,
        }
    }

    /// The previewed world-space position of whichever vertex is being
    /// dragged (camera, wall vertex, or door endpoint), if any. `None` for FOV
    /// previews (aim/range), which don't move a point.
    pub fn dragged_vertex_position(&self) -> Option<Point> {
        match self {
            DragPreview::Position { position, .. }
            | DragPreview::WallVertex { position, .. }
            | DragPreview::DoorEndpoint { position, .. } => Some(position.clone()),
            DragPreview::Fov { .. } | DragPreview::None => None,
        }
    }

    /// The placed cameras with this preview applied, so the canvas renders the
    /// in-progress gesture instead of the stored map.
    pub fn apply_to_cameras(&self, cameras: &[MapCamera]) -> Vec<MapCamera> {
        cameras
            .iter()
            .map(|camera| {
                let mut camera = camera.clone();
                match self {
                    DragPreview::Position {
                        camera_id,
                        position,
                    } if *camera_id == camera.camera_id => {
                        camera.position = position.clone();
                    }
                    DragPreview::Fov { camera_id, fov } if *camera_id == camera.camera_id => {
                        camera.fov = fov.clone();
                    }
                    _ => {}
                }
                camera
            })
            .collect()
    }

    /// The placed walls with this preview's vertex drag applied, same shape as
    /// [`Self::apply_to_cameras`].
    pub fn apply_to_walls(&self, walls: &[MapWall]) -> Vec<MapWall> {
        walls
            .iter()
            .map(|wall| {
                let mut wall = wall.clone();
                if let DragPreview::WallVertex {
                    wall_id,
                    vertex_index,
                    position,
                } = self
                    && *wall_id == wall.id
                    && let Some(v) = wall.vertices.get_mut(*vertex_index)
                {
                    *v = position.clone();
                }
                wall
            })
            .collect()
    }

    /// The placed doors with this preview's endpoint drag applied, same shape
    /// as [`Self::apply_to_walls`].
    pub fn apply_to_doors(&self, doors: &[MapDoor]) -> Vec<MapDoor> {
        doors
            .iter()
            .map(|door| {
                let mut door = door.clone();
                if let DragPreview::DoorEndpoint {
                    door_id,
                    which,
                    position,
                } = self
                    && *door_id == door.id
                {
                    match which {
                        Endpoint::Start => door.start = position.clone(),
                        Endpoint::End => door.end = position.clone(),
                    }
                }
                door
            })
            .collect()
    }
}

/// The active editing tool. `Select` is the default/neutral tool (click to
/// select, drag to move/pan); other variants arm a placement/drawing
/// interaction. `EditToolbar` matches on it directly to derive each button's
/// active state, rather than the caller pre-computing a bool per tool.
#[derive(Clone, PartialEq, Default)]
pub enum Tool {
    #[default]
    Select,
    /// A camera id chosen from the picker, awaiting a placement tap.
    PlaceCamera(String),
    /// Drawing a wall path. `vertices` accumulates world-space points as the
    /// user clicks; nothing is committed to the map until the path finishes.
    DrawWall { vertices: Vec<Point> },
    /// Placing a door: `start` is `None` until the first of two clicks, then
    /// `Some(point)` awaiting the second click to complete it.
    PlaceDoor { start: Option<Point> },
}

/// What's currently selected in edit mode, for the contextual inspector.
#[derive(Clone, PartialEq)]
pub enum Selection {
    Camera(String),
    Wall(String),
    Door(String),
}

/// What pressing Escape should do given the current tool / picker state.
/// Produced by [`escape_transition`]; the component applies it to its signals.
#[derive(Clone, PartialEq)]
pub enum EscapeAction {
    /// Back out one stage of an in-progress placement / drawing tool.
    SetTool(Tool),
    /// Close the camera-picker sheet.
    ClosePicker,
    /// Nothing left to unwind: clear the selection and exit edit mode,
    /// mirroring the "Done" button's reset.
    ExitEditMode,
}

/// Escape's back-out cascade, innermost state first: cancel the active
/// placement tool (no commit — same free-cancel semantics as switching back to
/// Select). Door placement gets a two-stage cancel: the first Escape backs out
/// of the pending second click (dropping the placed start point but staying in
/// the tool), and a second Escape then fully exits to Select — smoother than
/// losing the whole in-progress placement on one keypress. Choosing a camera
/// from the picker also arms `PlaceCamera`, so Escape backs that out to Select
/// too; and if the picker sheet itself is still open (tool hasn't left Select
/// yet), Escape closes it. Once back on a plain Select with nothing else to
/// unwind, a further Escape exits edit mode entirely.
pub fn escape_transition(tool: &Tool, picker_open: bool) -> EscapeAction {
    match tool {
        Tool::DrawWall { .. } => EscapeAction::SetTool(Tool::Select),
        Tool::PlaceDoor { start: Some(_) } => {
            EscapeAction::SetTool(Tool::PlaceDoor { start: None })
        }
        Tool::PlaceDoor { start: None } => EscapeAction::SetTool(Tool::Select),
        Tool::PlaceCamera(_) => EscapeAction::SetTool(Tool::Select),
        Tool::Select if picker_open => EscapeAction::ClosePicker,
        Tool::Select => EscapeAction::ExitEditMode,
    }
}

#[cfg(test)]
mod tests {
    use shield_models::{DoorSwing, WallColor};

    use super::*;

    fn point(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    #[test]
    fn escape_cascades_from_placement_tools_to_select() {
        assert!(matches!(
            escape_transition(
                &Tool::DrawWall {
                    vertices: vec![point(0, 0)]
                },
                false
            ),
            EscapeAction::SetTool(Tool::Select)
        ));
        assert!(matches!(
            escape_transition(&Tool::PlaceCamera("c".into()), false),
            EscapeAction::SetTool(Tool::Select)
        ));
    }

    #[test]
    fn escape_backs_door_placement_out_in_two_stages() {
        // First Escape drops the pending start point but keeps the tool armed.
        assert!(matches!(
            escape_transition(
                &Tool::PlaceDoor {
                    start: Some(point(1, 2))
                },
                false
            ),
            EscapeAction::SetTool(Tool::PlaceDoor { start: None })
        ));
        // Second Escape exits to Select.
        assert!(matches!(
            escape_transition(&Tool::PlaceDoor { start: None }, false),
            EscapeAction::SetTool(Tool::Select)
        ));
    }

    #[test]
    fn escape_on_select_closes_picker_then_exits_edit_mode() {
        assert!(matches!(
            escape_transition(&Tool::Select, true),
            EscapeAction::ClosePicker
        ));
        assert!(matches!(
            escape_transition(&Tool::Select, false),
            EscapeAction::ExitEditMode
        ));
    }

    fn camera(id: &str, x: i32, y: i32) -> MapCamera {
        MapCamera {
            camera_id: id.into(),
            position: point(x, y),
            fov: FieldOfView {
                direction_deg: 0,
                angle_deg: 70,
                range: 500,
            },
        }
    }

    #[test]
    fn apply_to_cameras_moves_only_the_previewed_camera() {
        let cameras = [camera("a", 0, 0), camera("b", 10, 10)];
        let preview = DragPreview::Position {
            camera_id: "a".into(),
            position: point(5, 6),
        };
        let shown = preview.apply_to_cameras(&cameras);
        assert_eq!((shown[0].position.x, shown[0].position.y), (5, 6));
        assert_eq!((shown[1].position.x, shown[1].position.y), (10, 10));
    }

    #[test]
    fn apply_to_cameras_swaps_fov_for_fov_preview() {
        let cameras = [camera("a", 0, 0)];
        let fov = FieldOfView {
            direction_deg: 90,
            angle_deg: 45,
            range: 200,
        };
        let preview = DragPreview::Fov {
            camera_id: "a".into(),
            fov: fov.clone(),
        };
        let shown = preview.apply_to_cameras(&cameras);
        assert!(shown[0].fov == fov);
    }

    #[test]
    fn apply_to_walls_moves_only_the_previewed_vertex() {
        let wall = MapWall {
            id: "w".into(),
            vertices: vec![point(0, 0), point(10, 0)],
            closed: false,
            color: WallColor::default(),
        };
        let preview = DragPreview::WallVertex {
            wall_id: "w".into(),
            vertex_index: 1,
            position: point(20, 5),
        };
        let shown = preview.apply_to_walls(&[wall]);
        assert_eq!((shown[0].vertices[0].x, shown[0].vertices[0].y), (0, 0));
        assert_eq!((shown[0].vertices[1].x, shown[0].vertices[1].y), (20, 5));
    }

    #[test]
    fn apply_to_walls_ignores_out_of_range_vertex_index() {
        let wall = MapWall {
            id: "w".into(),
            vertices: vec![point(0, 0)],
            closed: false,
            color: WallColor::default(),
        };
        let preview = DragPreview::WallVertex {
            wall_id: "w".into(),
            vertex_index: 5,
            position: point(20, 5),
        };
        let shown = preview.apply_to_walls(&[wall]);
        assert_eq!((shown[0].vertices[0].x, shown[0].vertices[0].y), (0, 0));
    }

    #[test]
    fn apply_to_doors_moves_only_the_previewed_endpoint() {
        let door = MapDoor {
            id: "d".into(),
            start: point(0, 0),
            end: point(100, 0),
            swing: DoorSwing::default(),
        };
        let preview = DragPreview::DoorEndpoint {
            door_id: "d".into(),
            which: Endpoint::End,
            position: point(100, 30),
        };
        let shown = preview.apply_to_doors(&[door]);
        assert_eq!((shown[0].start.x, shown[0].start.y), (0, 0));
        assert_eq!((shown[0].end.x, shown[0].end.y), (100, 30));
    }
}
