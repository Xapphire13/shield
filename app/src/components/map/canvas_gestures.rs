//! The canvas gesture state machine: what a pointer down / move / up or a
//! pinch does to the active [`Gesture`], [`DragPreview`], and [`Tool`].
//!
//! Everything here is a pure transition — inputs in, a described outcome out —
//! so the drag/placement behavior is unit-testable without a DOM. `MapView`'s
//! event handlers stay thin: convert the event to canvas coordinates, call the
//! transition, apply the outcome to its signals / commit callbacks.

use dioxus::prelude::*;
use shield_models::{DoorSwing, FieldOfView, MapCamera, MapDoor, MapWall, Point, WallColor};

use crate::components::map::geometry::{apply_drag_delta, bearing_to, distance};
use crate::components::map::interaction::{DragPreview, Gesture, Tool};
use crate::components::map::map_door::Endpoint;
use crate::components::map::viewport::Viewport;

/// Default field-of-view applied to a freshly placed camera.
const DEFAULT_FOV: FieldOfView = FieldOfView {
    direction_deg: 0,
    angle_deg: 70,
    range: 500,
};

/// Smallest range a camera cone may be dragged to (centimeters).
const MIN_RANGE_CM: i32 = 50;

/// Screen-pixel radius (not world-space) within which a click near the
/// first vertex of an in-progress wall draft closes the path into a loop.
/// Screen-space, not world-space, so the target feels the same size
/// regardless of zoom level.
pub const CLOSE_LOOP_HIT_RADIUS_PX: f64 = 14.0;

/// Screen-pixel radius (not world-space) within which the last two vertices
/// of a just-finished wall draft are treated as the same double-click point
/// (see the double-click-finish dedup in [`finish_wall_draft`]). Screen-space
/// for the same reason as `CLOSE_LOOP_HIT_RADIUS_PX`: a world-space threshold
/// would need to be huge at low zoom and negligible at high zoom to represent
/// "the same physical click" either way.
const DOUBLE_CLICK_DEDUP_RADIUS_PX: f64 = 6.0;

/// Convert a pointer event to canvas-relative pixels using a cached canvas
/// origin (the canvas's viewport-relative top-left).
///
/// All pointer math must share one coordinate space, but `element_coordinates`
/// is relative to whichever child element is under the pointer — during a drag
/// the pointer crosses the markers, cones, handles and grid, so its origin keeps
/// changing. `client_coordinates` is viewport-relative and target-independent;
/// subtracting the cached canvas origin yields a stable canvas-relative point
/// that every gesture (pan / move / aim / range / wheel) can rely on.
pub fn canvas_xy(evt: &PointerData, origin: (f64, f64)) -> (f64, f64) {
    let client = evt.client_coordinates();
    (client.x - origin.0, client.y - origin.1)
}

/// What a pointer down at canvas point `(cx, cy)` does for the armed placement
/// tool, or `None` when the tool is `Select` (the caller then falls through to
/// its selection / pan handling).
pub enum ToolDownAction {
    /// Commit a freshly placed camera (the caller also selects it).
    PlaceCamera(MapCamera),
    /// The click landed on the draft's first vertex: commit the path as a
    /// closed loop.
    CloseWallLoop(MapWall),
    /// Append the clicked point to the wall draft.
    ExtendWallDraft(Vec<Point>),
    /// First of the two door clicks: arm the second.
    SetDoorStart(Point),
    /// Second door click: commit the completed door.
    PlaceDoor(MapDoor),
}

/// Pointer-down transition for the placement/drawing tools.
pub fn tool_pointer_down(
    tool: Tool,
    viewport: Viewport,
    cx: f64,
    cy: f64,
) -> Option<ToolDownAction> {
    let (wx, wy) = viewport.screen_to_world(cx, cy);
    let world_point = Point {
        x: wx.round() as i32,
        y: wy.round() as i32,
    };
    match tool {
        Tool::Select => None,
        Tool::PlaceCamera(camera_id) => Some(ToolDownAction::PlaceCamera(MapCamera {
            camera_id,
            position: world_point,
            fov: DEFAULT_FOV,
        })),
        Tool::DrawWall { mut vertices } => {
            // Close-loop hit-test: only meaningful once there's an actual loop
            // to close (need >= 3 vertices before "closing" makes geometric
            // sense — with fewer points it would just double back on itself).
            if vertices.len() >= 3 {
                let (v0_sx, v0_sy) =
                    viewport.world_to_screen(vertices[0].x as f64, vertices[0].y as f64);
                if distance(cx, cy, v0_sx, v0_sy) <= CLOSE_LOOP_HIT_RADIUS_PX {
                    return Some(ToolDownAction::CloseWallLoop(MapWall {
                        id: uuid::Uuid::new_v4().to_string(),
                        vertices,
                        closed: true,
                        color: WallColor::default(),
                    }));
                }
            }
            vertices.push(world_point);
            Some(ToolDownAction::ExtendWallDraft(vertices))
        }
        Tool::PlaceDoor { start: None } => Some(ToolDownAction::SetDoorStart(world_point)),
        // A door is always exactly two points, so the second click both
        // finishes AND commits in one step — unlike wall drafting there is no
        // separate "finish" affordance. The newly placed door is deliberately
        // left unselected, same as a newly-drawn wall: selecting it requires a
        // follow-up tap on the opening line.
        Tool::PlaceDoor { start: Some(start) } => Some(ToolDownAction::PlaceDoor(MapDoor {
            id: uuid::Uuid::new_v4().to_string(),
            start,
            end: world_point,
            swing: DoorSwing::default(),
        })),
    }
}

/// What a pointer move does to the active gesture.
pub enum PointerMoveOutcome {
    /// The active gesture doesn't react to pointer movement (idle / pinch,
    /// or a drag whose target element no longer exists).
    None,
    /// Pan the viewport by a screen-pixel delta and refresh the gesture's
    /// last-seen point.
    Pan { dx: f64, dy: f64, next: Gesture },
    /// Show `preview` locally (nothing committed). `next` refreshes the
    /// gesture's last-seen point for the delta-based drags; it is `None` for
    /// aim/range, which track the pointer absolutely and never change state.
    Preview {
        preview: DragPreview,
        next: Option<Gesture>,
    },
}

/// Pointer-move transition: how the active gesture responds to the pointer
/// reaching canvas point `(cx, cy)`.
///
/// Delta-based drags (camera body, wall vertex, door endpoint) continue from
/// the active preview's position when one exists, falling back to the stored
/// map data for the first move of the drag.
pub fn pointer_move_transition(
    gesture: &Gesture,
    preview: &DragPreview,
    viewport: Viewport,
    cameras: &[MapCamera],
    walls: &[MapWall],
    doors: &[MapDoor],
    cx: f64,
    cy: f64,
) -> PointerMoveOutcome {
    match gesture.clone() {
        Gesture::Pan { last_x, last_y } => PointerMoveOutcome::Pan {
            dx: cx - last_x,
            dy: cy - last_y,
            next: Gesture::Pan {
                last_x: cx,
                last_y: cy,
            },
        },
        Gesture::MoveCamera {
            camera_id,
            last_x,
            last_y,
        } => {
            let base = preview.position_for(&camera_id).or_else(|| {
                cameras
                    .iter()
                    .find(|c| c.camera_id == camera_id)
                    .map(|c| c.position.clone())
            });
            let Some(base) = base else {
                return PointerMoveOutcome::None;
            };
            let position = apply_drag_delta(base, cx, cy, last_x, last_y, viewport.zoom);
            PointerMoveOutcome::Preview {
                preview: DragPreview::Position {
                    camera_id: camera_id.clone(),
                    position,
                },
                next: Some(Gesture::MoveCamera {
                    camera_id,
                    last_x: cx,
                    last_y: cy,
                }),
            }
        }
        Gesture::AimCamera { camera_id } => {
            let Some(camera) = cameras.iter().find(|c| c.camera_id == camera_id) else {
                return PointerMoveOutcome::None;
            };
            let (wx, wy) = viewport.screen_to_world(cx, cy);
            let direction_deg =
                bearing_to(camera.position.x as f64, camera.position.y as f64, wx, wy);
            let fov = FieldOfView {
                direction_deg,
                ..preview.fov_for(&camera_id).unwrap_or(camera.fov.clone())
            };
            PointerMoveOutcome::Preview {
                preview: DragPreview::Fov { camera_id, fov },
                next: None,
            }
        }
        Gesture::RangeCamera { camera_id } => {
            let Some(camera) = cameras.iter().find(|c| c.camera_id == camera_id) else {
                return PointerMoveOutcome::None;
            };
            let (wx, wy) = viewport.screen_to_world(cx, cy);
            let dist = distance(camera.position.x as f64, camera.position.y as f64, wx, wy);
            let range = (dist.round() as i32).max(MIN_RANGE_CM);
            let fov = FieldOfView {
                range,
                ..preview.fov_for(&camera_id).unwrap_or(camera.fov.clone())
            };
            PointerMoveOutcome::Preview {
                preview: DragPreview::Fov { camera_id, fov },
                next: None,
            }
        }
        Gesture::MoveWallVertex {
            wall_id,
            vertex_index,
            last_x,
            last_y,
        } => {
            let base = preview.wall_vertex_for(&wall_id, vertex_index).or_else(|| {
                walls
                    .iter()
                    .find(|w| w.id == wall_id)
                    .and_then(|w| w.vertices.get(vertex_index).cloned())
            });
            let Some(base) = base else {
                return PointerMoveOutcome::None;
            };
            let position = apply_drag_delta(base, cx, cy, last_x, last_y, viewport.zoom);
            PointerMoveOutcome::Preview {
                preview: DragPreview::WallVertex {
                    wall_id: wall_id.clone(),
                    vertex_index,
                    position,
                },
                next: Some(Gesture::MoveWallVertex {
                    wall_id,
                    vertex_index,
                    last_x: cx,
                    last_y: cy,
                }),
            }
        }
        Gesture::MoveDoorEndpoint {
            door_id,
            which,
            last_x,
            last_y,
        } => {
            let base = preview.door_endpoint_for(&door_id, which).or_else(|| {
                doors.iter().find(|d| d.id == door_id).map(|d| match which {
                    Endpoint::Start => d.start.clone(),
                    Endpoint::End => d.end.clone(),
                })
            });
            let Some(base) = base else {
                return PointerMoveOutcome::None;
            };
            let position = apply_drag_delta(base, cx, cy, last_x, last_y, viewport.zoom);
            PointerMoveOutcome::Preview {
                preview: DragPreview::DoorEndpoint {
                    door_id: door_id.clone(),
                    which,
                    position,
                },
                next: Some(Gesture::MoveDoorEndpoint {
                    door_id,
                    which,
                    last_x: cx,
                    last_y: cy,
                }),
            }
        }
        Gesture::None | Gesture::Pinch { .. } => PointerMoveOutcome::None,
    }
}

/// The single map edit a completed gesture commits on release.
pub enum MapCommit {
    MoveCamera {
        camera_id: String,
        position: Point,
    },
    AimCamera {
        camera_id: String,
        fov: FieldOfView,
    },
    MoveWallVertex {
        wall_id: String,
        vertex_index: usize,
        position: Point,
    },
    MoveDoorEndpoint {
        door_id: String,
        start: bool,
        position: Point,
    },
}

/// Pointer-up transition: the edit to commit for the gesture that just ended,
/// or `None` when there is nothing to commit (pan / pinch / no preview built).
pub fn pointer_up_commit(gesture: &Gesture, preview: &DragPreview) -> Option<MapCommit> {
    match gesture.clone() {
        Gesture::MoveCamera { camera_id, .. } => {
            if let DragPreview::Position { position, .. } = preview.clone() {
                Some(MapCommit::MoveCamera {
                    camera_id,
                    position,
                })
            } else {
                None
            }
        }
        Gesture::AimCamera { camera_id } | Gesture::RangeCamera { camera_id } => {
            if let DragPreview::Fov { fov, .. } = preview.clone() {
                Some(MapCommit::AimCamera { camera_id, fov })
            } else {
                None
            }
        }
        Gesture::MoveWallVertex {
            wall_id,
            vertex_index,
            ..
        } => {
            if let DragPreview::WallVertex { position, .. } = preview.clone() {
                Some(MapCommit::MoveWallVertex {
                    wall_id,
                    vertex_index,
                    position,
                })
            } else {
                None
            }
        }
        Gesture::MoveDoorEndpoint { door_id, which, .. } => {
            if let DragPreview::DoorEndpoint { position, .. } = preview.clone() {
                Some(MapCommit::MoveDoorEndpoint {
                    door_id,
                    start: which == Endpoint::Start,
                    position,
                })
            } else {
                None
            }
        }
        Gesture::None | Gesture::Pan { .. } | Gesture::Pinch { .. } => None,
    }
}

/// Finish an open wall draft on double-click, returning the wall to commit
/// (or `None` when the draft is too short to form a segment).
///
/// A double-click is physically two separate clicks in quick succession, both
/// of which already ran through the pointer-down handler and each pushed a
/// vertex at (approximately) the same point. The trailing duplicate is dropped
/// so finishing a path doesn't leave a spurious extra vertex at the exact spot
/// the user double-clicked. The dedup threshold is screen-space, not
/// world-space: both points came from clicks at essentially the same physical
/// pixel, but that maps to wildly different world-cm distances depending on
/// zoom (e.g. 1px is 50 world-cm at minimum zoom), so a fixed world-space
/// threshold would either over- or under-fire depending on zoom level.
pub fn finish_wall_draft(mut vertices: Vec<Point>, viewport: Viewport) -> Option<MapWall> {
    if vertices.len() >= 2 {
        let last = vertices.len() - 1;
        let (last_sx, last_sy) =
            viewport.world_to_screen(vertices[last].x as f64, vertices[last].y as f64);
        let (prev_sx, prev_sy) =
            viewport.world_to_screen(vertices[last - 1].x as f64, vertices[last - 1].y as f64);
        if distance(last_sx, last_sy, prev_sx, prev_sy) < DOUBLE_CLICK_DEDUP_RADIUS_PX {
            vertices.pop();
        }
    }

    (vertices.len() >= 2).then(|| MapWall {
        id: uuid::Uuid::new_v4().to_string(),
        vertices,
        closed: false,
        color: WallColor::default(),
    })
}

/// Begin a two-finger pinch from the fingers' current positions.
pub fn pinch_start(a: (f64, f64), b: (f64, f64)) -> Gesture {
    Gesture::Pinch {
        last_distance: distance(a.0, a.1, b.0, b.1),
    }
}

/// One step of an in-progress pinch: the zoom factor and screen anchor to
/// apply (factor is `None` when the last distance was degenerate), plus the
/// refreshed gesture.
pub struct PinchUpdate {
    pub factor: Option<f64>,
    pub anchor: (f64, f64),
    pub next: Gesture,
}

/// Pinch-move transition from the fingers' current positions.
pub fn pinch_move(last_distance: f64, a: (f64, f64), b: (f64, f64)) -> PinchUpdate {
    let dist = distance(a.0, a.1, b.0, b.1);
    PinchUpdate {
        factor: (last_distance > 0.0).then(|| dist / last_distance),
        anchor: ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
        next: Gesture::Pinch {
            last_distance: dist,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    /// Identity-ish viewport: zoom 1, no pan, so screen px == world cm.
    fn unit_viewport() -> Viewport {
        Viewport {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }

    fn camera(id: &str, x: i32, y: i32) -> MapCamera {
        MapCamera {
            camera_id: id.into(),
            position: point(x, y),
            fov: DEFAULT_FOV,
        }
    }

    #[test]
    fn select_tool_does_not_consume_pointer_down() {
        assert!(tool_pointer_down(Tool::Select, unit_viewport(), 10.0, 10.0).is_none());
    }

    #[test]
    fn place_camera_tool_places_at_world_point_with_default_fov() {
        let action =
            tool_pointer_down(Tool::PlaceCamera("cam".into()), unit_viewport(), 40.0, 60.0);
        let Some(ToolDownAction::PlaceCamera(placed)) = action else {
            panic!("expected PlaceCamera");
        };
        assert_eq!(placed.camera_id, "cam");
        assert_eq!((placed.position.x, placed.position.y), (40, 60));
        assert!(placed.fov == DEFAULT_FOV);
    }

    #[test]
    fn draw_wall_appends_vertices_until_loop_closes() {
        // First click: appends.
        let action = tool_pointer_down(
            Tool::DrawWall { vertices: vec![] },
            unit_viewport(),
            0.0,
            0.0,
        );
        let Some(ToolDownAction::ExtendWallDraft(vertices)) = action else {
            panic!("expected ExtendWallDraft");
        };
        assert_eq!(vertices.len(), 1);

        // With >= 3 vertices, a click within the hit radius of the first
        // vertex closes the loop instead of appending.
        let draft = vec![point(0, 0), point(100, 0), point(100, 100)];
        let action = tool_pointer_down(
            Tool::DrawWall {
                vertices: draft.clone(),
            },
            unit_viewport(),
            CLOSE_LOOP_HIT_RADIUS_PX - 1.0,
            0.0,
        );
        let Some(ToolDownAction::CloseWallLoop(wall)) = action else {
            panic!("expected CloseWallLoop");
        };
        assert!(wall.closed);
        assert_eq!(wall.vertices.len(), 3);

        // Outside the radius: appends a fourth vertex.
        let action = tool_pointer_down(
            Tool::DrawWall { vertices: draft },
            unit_viewport(),
            CLOSE_LOOP_HIT_RADIUS_PX + 1.0,
            0.0,
        );
        assert!(matches!(
            action,
            Some(ToolDownAction::ExtendWallDraft(v)) if v.len() == 4
        ));
    }

    #[test]
    fn two_vertex_draft_cannot_close_into_a_loop() {
        // With only 2 vertices a click on the first vertex appends (a
        // "closed" 2-point wall would just double back on itself).
        let action = tool_pointer_down(
            Tool::DrawWall {
                vertices: vec![point(0, 0), point(100, 0)],
            },
            unit_viewport(),
            0.0,
            0.0,
        );
        assert!(matches!(
            action,
            Some(ToolDownAction::ExtendWallDraft(v)) if v.len() == 3
        ));
    }

    #[test]
    fn door_placement_is_two_clicks() {
        let action =
            tool_pointer_down(Tool::PlaceDoor { start: None }, unit_viewport(), 10.0, 20.0);
        let Some(ToolDownAction::SetDoorStart(start)) = action else {
            panic!("expected SetDoorStart");
        };
        assert_eq!((start.x, start.y), (10, 20));

        let action = tool_pointer_down(
            Tool::PlaceDoor {
                start: Some(point(10, 20)),
            },
            unit_viewport(),
            110.0,
            20.0,
        );
        let Some(ToolDownAction::PlaceDoor(door)) = action else {
            panic!("expected PlaceDoor");
        };
        assert_eq!((door.start.x, door.start.y), (10, 20));
        assert_eq!((door.end.x, door.end.y), (110, 20));
    }

    #[test]
    fn pan_move_pans_by_screen_delta() {
        let outcome = pointer_move_transition(
            &Gesture::Pan {
                last_x: 10.0,
                last_y: 10.0,
            },
            &DragPreview::None,
            unit_viewport(),
            &[],
            &[],
            &[],
            25.0,
            (-5.0),
        );
        let PointerMoveOutcome::Pan { dx, dy, next } = outcome else {
            panic!("expected Pan");
        };
        assert_eq!((dx, dy), (15.0, -15.0));
        assert!(
            matches!(next, Gesture::Pan { last_x, last_y } if last_x == 25.0 && last_y == -5.0)
        );
    }

    #[test]
    fn camera_drag_previews_from_stored_position_then_from_preview() {
        let cameras = [camera("cam", 100, 100)];
        let gesture = Gesture::MoveCamera {
            camera_id: "cam".into(),
            last_x: 0.0,
            last_y: 0.0,
        };
        // First move: base comes from the stored map.
        let outcome = pointer_move_transition(
            &gesture,
            &DragPreview::None,
            unit_viewport(),
            &cameras,
            &[],
            &[],
            10.0,
            0.0,
        );
        let PointerMoveOutcome::Preview { preview, next } = outcome else {
            panic!("expected Preview");
        };
        let pos = preview.position_for("cam").unwrap();
        assert_eq!((pos.x, pos.y), (110, 100));
        assert!(next.is_some());

        // Second move: base continues from the preview, not the stored map.
        let next = next.unwrap();
        let outcome = pointer_move_transition(
            &next,
            &preview,
            unit_viewport(),
            &cameras,
            &[],
            &[],
            15.0,
            0.0,
        );
        let PointerMoveOutcome::Preview { preview, .. } = outcome else {
            panic!("expected Preview");
        };
        let pos = preview.position_for("cam").unwrap();
        assert_eq!((pos.x, pos.y), (115, 100));
    }

    #[test]
    fn camera_drag_for_unknown_camera_is_inert() {
        let outcome = pointer_move_transition(
            &Gesture::MoveCamera {
                camera_id: "ghost".into(),
                last_x: 0.0,
                last_y: 0.0,
            },
            &DragPreview::None,
            unit_viewport(),
            &[],
            &[],
            &[],
            10.0,
            0.0,
        );
        assert!(matches!(outcome, PointerMoveOutcome::None));
    }

    #[test]
    fn aim_drag_rotates_cone_toward_pointer() {
        let cameras = [camera("cam", 0, 0)];
        // Pointer due East of the camera -> bearing 90.
        let outcome = pointer_move_transition(
            &Gesture::AimCamera {
                camera_id: "cam".into(),
            },
            &DragPreview::None,
            unit_viewport(),
            &cameras,
            &[],
            &[],
            100.0,
            0.0,
        );
        let PointerMoveOutcome::Preview { preview, next } = outcome else {
            panic!("expected Preview");
        };
        let fov = preview.fov_for("cam").unwrap();
        assert_eq!(fov.direction_deg, 90);
        // Angle and range are carried over untouched.
        assert_eq!(fov.angle_deg, DEFAULT_FOV.angle_deg);
        assert_eq!(fov.range, DEFAULT_FOV.range);
        // Aim doesn't track a last-seen point, so no gesture refresh.
        assert!(next.is_none());
    }

    #[test]
    fn range_drag_clamps_to_minimum() {
        let cameras = [camera("cam", 0, 0)];
        let outcome = pointer_move_transition(
            &Gesture::RangeCamera {
                camera_id: "cam".into(),
            },
            &DragPreview::None,
            unit_viewport(),
            &cameras,
            &[],
            &[],
            10.0,
            0.0,
        );
        let PointerMoveOutcome::Preview { preview, .. } = outcome else {
            panic!("expected Preview");
        };
        assert_eq!(preview.fov_for("cam").unwrap().range, MIN_RANGE_CM);
    }

    #[test]
    fn pointer_up_commits_matching_preview_only() {
        // A camera drag with a position preview commits a move.
        let commit = pointer_up_commit(
            &Gesture::MoveCamera {
                camera_id: "cam".into(),
                last_x: 0.0,
                last_y: 0.0,
            },
            &DragPreview::Position {
                camera_id: "cam".into(),
                position: point(5, 6),
            },
        );
        assert!(matches!(
            commit,
            Some(MapCommit::MoveCamera { camera_id, position })
                if camera_id == "cam" && position.x == 5 && position.y == 6
        ));

        // No preview built (pointer never moved): nothing commits.
        assert!(
            pointer_up_commit(
                &Gesture::MoveCamera {
                    camera_id: "cam".into(),
                    last_x: 0.0,
                    last_y: 0.0,
                },
                &DragPreview::None,
            )
            .is_none()
        );

        // Pan commits nothing.
        assert!(
            pointer_up_commit(
                &Gesture::Pan {
                    last_x: 0.0,
                    last_y: 0.0
                },
                &DragPreview::None
            )
            .is_none()
        );
    }

    #[test]
    fn pointer_up_maps_door_endpoint_to_start_flag() {
        let commit = pointer_up_commit(
            &Gesture::MoveDoorEndpoint {
                door_id: "d".into(),
                which: Endpoint::End,
                last_x: 0.0,
                last_y: 0.0,
            },
            &DragPreview::DoorEndpoint {
                door_id: "d".into(),
                which: Endpoint::End,
                position: point(1, 2),
            },
        );
        assert!(matches!(
            commit,
            Some(MapCommit::MoveDoorEndpoint { start: false, .. })
        ));
    }

    #[test]
    fn finish_wall_draft_drops_trailing_double_click_duplicate() {
        // The double-click's two pointer-downs pushed two vertices at nearly
        // the same point; the trailing one is dropped.
        let wall = finish_wall_draft(
            vec![point(0, 0), point(100, 0), point(100, 1)],
            unit_viewport(),
        )
        .unwrap();
        assert_eq!(wall.vertices.len(), 2);
        assert!(!wall.closed);

        // Distinct last vertex survives.
        let wall = finish_wall_draft(
            vec![point(0, 0), point(100, 0), point(100, 50)],
            unit_viewport(),
        )
        .unwrap();
        assert_eq!(wall.vertices.len(), 3);
    }

    #[test]
    fn finish_wall_draft_rejects_too_short_drafts() {
        assert!(finish_wall_draft(vec![], unit_viewport()).is_none());
        assert!(finish_wall_draft(vec![point(0, 0)], unit_viewport()).is_none());
        // Two vertices that dedup down to one also can't form a segment.
        assert!(finish_wall_draft(vec![point(0, 0), point(1, 0)], unit_viewport()).is_none());
    }

    #[test]
    fn pinch_zooms_by_distance_ratio_around_midpoint() {
        let gesture = pinch_start((0.0, 0.0), (100.0, 0.0));
        let Gesture::Pinch { last_distance } = gesture else {
            panic!("expected Pinch");
        };
        assert_eq!(last_distance, 100.0);

        let update = pinch_move(last_distance, (0.0, 0.0), (200.0, 0.0));
        assert_eq!(update.factor, Some(2.0));
        assert_eq!(update.anchor, (100.0, 0.0));
        assert!(matches!(update.next, Gesture::Pinch { last_distance } if last_distance == 200.0));

        // Degenerate previous distance produces no zoom factor.
        assert_eq!(pinch_move(0.0, (0.0, 0.0), (10.0, 0.0)).factor, None);
    }
}
