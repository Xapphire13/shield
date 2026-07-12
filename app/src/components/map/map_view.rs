use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdCornerUpLeft, LdCornerUpRight};
use shield_models::{DoorSwing, FieldOfView, MapCamera, MapDoor, MapWall, Point, WallColor};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::components::layout::TopBar;
use crate::components::map::camera_info::CameraInfo;
use crate::components::map::camera_inspector::CameraInspector;
use crate::components::map::door_inspector::DoorInspector;
use crate::components::map::edit_toolbar::{CameraPicker, EditToolbar};
use crate::components::map::geometry::{
    apply_drag_delta, bearing_to, content_bounds, distance, fully_contains_bounds,
};
use crate::components::map::map_camera::{MARKER_RADIUS_CM, MapCameraMarker};
use crate::components::map::map_door::{Endpoint, MapDoorMarker};
use crate::components::map::map_wall::MapWallPath;
use crate::components::map::minimap::Minimap;
use crate::components::map::unplaced_badge::UnplacedBadge;
use crate::components::map::viewport::{BUTTON_ZOOM_STEP, Viewport, WHEEL_ZOOM_STEP};
use crate::components::map::wall_inspector::WallInspector;
use crate::components::map::zoom_controls::ZoomControls;
use crate::hooks::{UseCamerasResult, UseMapResult, use_cameras, use_map};

/// The single map edited in v1. The service lazily returns an empty map for any
/// id, so a fixed default is sufficient until multi-map UI exists.
const DEFAULT_MAP_ID: &str = "default";

/// DOM id of the canvas frame element, used to locate it for measurement.
const CANVAS_FRAME_ID: &str = style::canvas_frame;

/// Look up the canvas frame element in the DOM by its id.
fn canvas_frame_element() -> Option<web_sys::Element> {
    web_sys::window()?
        .document()?
        .get_element_by_id(CANVAS_FRAME_ID)
}

/// Read the canvas frame's bounding rect as `(left, top, width, height)` in
/// viewport pixels, or `None` if it isn't in the DOM yet.
fn canvas_frame_rect() -> Option<(f64, f64, f64, f64)> {
    let rect = canvas_frame_element()?.get_bounding_client_rect();
    Some((rect.left(), rect.top(), rect.width(), rect.height()))
}

/// Run `f` after the browser has applied layout for the current frame, using a
/// double `requestAnimationFrame` (one frame to apply layout, one to be safe).
/// Each callback is one-shot, so `Closure::once_into_js` is used to hand it to
/// the browser without manual lifetime bookkeeping.
fn after_next_layout(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let inner = Closure::once_into_js(f);
    let outer = Closure::once_into_js(move || {
        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(inner.unchecked_ref());
        }
    });
    let _ = window.request_animation_frame(outer.unchecked_ref());
}

/// Default field-of-view applied to a freshly placed camera.
const DEFAULT_FOV: FieldOfView = FieldOfView {
    direction_deg: 0,
    angle_deg: 70,
    range: 500,
};

stylance::import_crate_style!(style, "src/components/map/map_view.module.css");

/// Smallest range a camera cone may be dragged to (centimeters).
const MIN_RANGE_CM: i32 = 50;

/// Screen-pixel radius (not world-space) within which a click near the
/// first vertex of an in-progress wall draft closes the path into a loop.
/// Screen-space, not world-space, so the target feels the same size
/// regardless of zoom level.
const CLOSE_LOOP_HIT_RADIUS_PX: f64 = 14.0;

/// Screen-pixel radius (not world-space) within which the last two vertices
/// of a just-finished wall draft are treated as the same double-click point
/// (see the double-click-finish dedup below). Screen-space for the same
/// reason as `CLOSE_LOOP_HIT_RADIUS_PX`: a world-space threshold would need
/// to be huge at low zoom and negligible at high zoom to represent "the same
/// physical click" either way.
const DOUBLE_CLICK_DEDUP_RADIUS_PX: f64 = 6.0;

/// Active gesture being tracked across pointer/touch events.
///
/// Pan starts on empty canvas; the camera-manipulation gestures start on a
/// marker / handle (which stops propagation so the canvas pan handler does not
/// also fire — this is the target-based disambiguation). Manipulation gestures
/// preview locally and commit exactly one edit on release.
#[derive(Clone, PartialEq)]
enum Gesture {
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
    fn label(&self) -> &'static str {
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
enum DragPreview {
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

/// The active editing tool. `Select` is the default/neutral tool (click to
/// select, drag to move/pan); other variants arm a placement/drawing
/// interaction. `pub(crate)` so `EditToolbar` can match on it directly to
/// derive each button's active state, rather than the caller pre-computing a
/// bool per tool.
#[derive(Clone, PartialEq, Default)]
pub(crate) enum Tool {
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
enum Selection {
    Camera(String),
    Wall(String),
    Door(String),
}

/// Convert a pointer event to canvas-relative pixels using a cached canvas
/// origin (the canvas's viewport-relative top-left).
///
/// All pointer math must share one coordinate space, but `element_coordinates`
/// is relative to whichever child element is under the pointer — during a drag
/// the pointer crosses the markers, cones, handles and grid, so its origin keeps
/// changing. `client_coordinates` is viewport-relative and target-independent;
/// subtracting the cached canvas origin yields a stable canvas-relative point
/// that every gesture (pan / move / aim / range / wheel) can rely on.
fn canvas_xy(evt: &PointerData, origin: (f64, f64)) -> (f64, f64) {
    let client = evt.client_coordinates();
    (client.x - origin.0, client.y - origin.1)
}

/// Map host: live data, pan/zoom viewport, and a full edit experience (place /
/// select / move / aim / inspect / delete). Outside edit mode it is a read-only,
/// pan/zoom-able canvas.
#[component]
pub fn MapView() -> Element {
    let UseMapResult {
        map,
        loading: map_loading,
        place_camera,
        move_camera,
        aim_camera,
        remove_camera,
        place_wall,
        move_wall_vertex,
        close_wall,
        recolor_wall,
        remove_wall,
        place_door,
        move_door_endpoint,
        flip_door_swing,
        remove_door,
        undo,
        redo,
        can_undo,
        can_redo,
        ..
    } = use_map(DEFAULT_MAP_ID.to_string());

    let UseCamerasResult {
        cameras: camera_list,
        loading: cameras_loading,
    } = use_cameras();

    let mut viewport = use_signal(Viewport::default);
    let mut gesture = use_signal(|| Gesture::None);
    let mut drag_preview = use_signal(|| DragPreview::None);

    let mut editing = use_signal(|| false);
    let mut selection = use_signal(|| None::<Selection>);
    // The placed camera whose read-only info popover is pinned in view mode, by
    // placed-reference id. Set by a tap/click and only used outside edit mode
    // (edit mode owns taps via the selection flow).
    let mut info_camera_id = use_signal(|| None::<String>);
    // The placed camera currently hovered in view mode (hover-capable devices
    // only; gated in CSS via `@media (hover: hover)`). Transient: cleared on
    // mouse-leave. A pinned (tapped) camera takes precedence over this.
    let mut hovered_camera_id = use_signal(|| None::<String>);
    // The active editing tool (Select, or a placement tool awaiting a canvas tap).
    let mut tool = use_signal(Tool::default);
    let mut picker_open = use_signal(|| false);
    // The pointer's canvas-relative screen position, tracked while any
    // placement tool is active so the coordinate readout can follow it.
    let mut cursor_pos = use_signal(|| None::<(f64, f64)>);

    // Cached canvas geometry from the frame's bounding rect: the viewport-relative
    // top-left (origin) drives canvas-relative pointer math (see `canvas_xy`), and
    // the size drives the initial fit-to-content. The rect is stable during a
    // drag, so the cached values stay correct.
    let mut canvas_origin = use_signal(|| (0.0_f64, 0.0_f64));
    let mut canvas_size = use_signal(|| (0.0_f64, 0.0_f64));
    // Whether the initial fit-to-content has been applied. Guards against
    // re-fitting on later edits / pans / zooms.
    let mut fitted = use_signal(|| false);

    // Measure the frame with a ResizeObserver rather than a one-shot mount read.
    // A mount-time read can run before the browser's first layout pass on a fresh
    // / deep-link load, measuring a not-yet-laid-out box; the observer instead
    // fires once *after* layout (fixing that case) and again on every size change,
    // so it also subsumes a window-resize listener and is the single source of
    // truth for both origin and size. The observer + its callback closure are
    // held in component state so they stay alive for the component's lifetime.
    let _observer = use_hook(|| {
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some((left, top, width, height)) = canvas_frame_rect() {
                canvas_origin.set((left, top));
                canvas_size.set((width, height));
            }
        });

        // The frame may not be in the DOM on the very first effect tick; retry on
        // the next layout frame if so.
        let observer = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()).ok();
        if let Some(observer) = &observer {
            if let Some(element) = canvas_frame_element() {
                observer.observe(&element);
            } else {
                let observer = observer.clone();
                after_next_layout(move || {
                    if let Some(element) = canvas_frame_element() {
                        observer.observe(&element);
                    }
                });
            }
        }

        // Keep both alive for the component's lifetime: dropping the closure
        // would invalidate the observer's callback, and dropping the observer
        // would stop notifications. `Rc` makes the stored state `Clone` (which
        // `use_hook` requires) without cloning the non-`Clone` closure.
        std::rc::Rc::new((observer, callback))
    });

    // Escape cancels the active placement tool (no commit — same free-cancel
    // semantics as switching back to Select). Door placement gets a
    // two-stage cancel: the first Escape backs out of the pending second
    // click (dropping the placed start point but staying in the tool), and a
    // second Escape then fully exits to Select — smoother than losing the
    // whole in-progress placement on one keypress. Choosing a camera from the
    // picker also arms `PlaceCamera`, so Escape backs that out to Select too;
    // and if the picker sheet itself is still open (tool hasn't left Select
    // yet), Escape closes it. Once back on a plain Select with nothing else
    // to unwind, a further Escape exits edit mode entirely, mirroring the
    // "Done" button's reset (clear selection, tool, and picker). Listened for
    // at the document level, same shape as the `ResizeObserver` hook above:
    // the closure and listener registration are kept alive together in
    // component state for the component's lifetime.
    let _keydown_listener = use_hook(|| {
        let callback = Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
            if let Ok(evt) = evt.dyn_into::<web_sys::KeyboardEvent>()
                && evt.key() == "Escape"
            {
                let current = tool.read().clone();
                match current {
                    Tool::DrawWall { .. } => tool.set(Tool::Select),
                    Tool::PlaceDoor { start: Some(_) } => tool.set(Tool::PlaceDoor { start: None }),
                    Tool::PlaceDoor { start: None } => tool.set(Tool::Select),
                    Tool::PlaceCamera(_) => tool.set(Tool::Select),
                    Tool::Select if *picker_open.read() => picker_open.set(false),
                    Tool::Select => {
                        selection.set(None);
                        editing.set(false);
                    }
                }
            }
        });

        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            let _ = document
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
        }

        // Keep the closure alive for the component's lifetime, same rationale
        // as the `ResizeObserver` hook: dropping it would invalidate the
        // registered listener's callback.
        std::rc::Rc::new(callback)
    });

    let placed = map.as_ref().map(|m| m.cameras.clone()).unwrap_or_default();
    let placed_walls = map.as_ref().map(|m| m.walls.clone()).unwrap_or_default();
    let placed_doors = map.as_ref().map(|m| m.doors.clone()).unwrap_or_default();

    // Fit and center the placed cameras, walls, and doors exactly once, after
    // layout has settled.
    //
    // On a hard / deep-link load the map data can arrive before the first styled
    // layout settles, so measuring synchronously here would fit against a
    // transient pre-layout size and the one-time `fitted` lock would freeze that
    // wrong value. To avoid that we defer to a post-layout animation frame and
    // measure the frame *fresh* there (double RAF: one frame to apply layout,
    // one to be safe), so the fit always uses the settled size. `fitted` still
    // locks it to a single fit (now the correct one) and never fights later user
    // pan/zoom; an empty map keeps the default viewport. `use_reactive` re-runs
    // this when the cameras, walls, or doors change (map load); guard reads use
    // `peek` so the effect depends only on `placed`/`placed_walls`/`placed_doors`.
    use_effect(use_reactive(
        (&placed, &placed_walls, &placed_doors),
        move |(placed, placed_walls, placed_doors)| {
            if *fitted.peek()
                || (placed.is_empty() && placed_walls.is_empty() && placed_doors.is_empty())
            {
                return;
            }
            after_next_layout(move || {
                let Some((_, _, width, height)) = canvas_frame_rect() else {
                    return;
                };
                if width > 0.0
                    && height > 0.0
                    && !*fitted.peek()
                    && let Some(bounds) = content_bounds(&placed, &placed_walls, &placed_doors)
                {
                    viewport.set(Viewport::fit_to_content(bounds, width, height));
                    fitted.set(true);
                }
            });
        },
    ));

    // Resolve a placed camera's display name, or `None` when its reference is an
    // orphan (underlying camera deleted).
    let name_for = {
        let camera_list = camera_list.clone();
        move |camera_id: &str| {
            camera_list
                .iter()
                .find(|c| c.id == camera_id)
                .map(|c| c.name.clone())
        }
    };

    // Cameras not yet on the map, offered by the picker.
    let unplaced: Vec<shield_models::Camera> = camera_list
        .iter()
        .filter(|c| !placed.iter().any(|p| p.camera_id == c.id))
        .cloned()
        .collect();

    // Apply any active preview so the canvas reflects the in-progress gesture.
    let preview = drag_preview.read().clone();
    let display_cameras: Vec<MapCamera> = placed
        .iter()
        .map(|camera| {
            let mut camera = camera.clone();
            match &preview {
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
        .collect();

    // Apply any active vertex-drag preview so the canvas reflects the
    // in-progress gesture, same shape as `display_cameras` above.
    let display_walls: Vec<MapWall> = placed_walls
        .iter()
        .map(|wall| {
            let mut wall = wall.clone();
            if let DragPreview::WallVertex {
                wall_id,
                vertex_index,
                position,
            } = &preview
                && *wall_id == wall.id
                && let Some(v) = wall.vertices.get_mut(*vertex_index)
            {
                *v = position.clone();
            }
            wall
        })
        .collect();

    // Apply any active endpoint-drag preview so the canvas reflects the
    // in-progress gesture, same shape as `display_walls` above.
    let display_doors: Vec<MapDoor> = placed_doors
        .iter()
        .map(|door| {
            let mut door = door.clone();
            if let DragPreview::DoorEndpoint {
                door_id,
                which,
                position,
            } = &preview
                && *door_id == door.id
            {
                match which {
                    Endpoint::Start => door.start = position.clone(),
                    Endpoint::End => door.end = position.clone(),
                }
            }
            door
        })
        .collect();

    let transform = viewport.read().transform();
    let grid_spacing_cm = viewport.read().grid_spacing_cm();
    let selected_camera_id = selection.read().clone().and_then(|s| match s {
        Selection::Camera(id) => Some(id),
        _ => None,
    });
    let selected_wall_id = selection.read().clone().and_then(|s| match s {
        Selection::Wall(id) => Some(id),
        _ => None,
    });
    let selected_door_id = selection.read().clone().and_then(|s| match s {
        Selection::Door(id) => Some(id),
        _ => None,
    });
    let is_editing = *editing.read();
    let is_placing = matches!(*tool.read(), Tool::PlaceCamera(_) | Tool::PlaceDoor { .. });
    let is_drawing_wall = matches!(*tool.read(), Tool::DrawWall { .. });
    // Placed elements (cameras, walls) are only selectable/draggable with the
    // Select tool active and no placement picker open — not just "in edit
    // mode". `tool` alone isn't enough: picking a camera from `CameraPicker`
    // doesn't arm `Tool::PlaceCamera` until a camera is actually chosen, so
    // `tool` still reads `Select` for the entire time the picker sheet is up,
    // and without this check elements stay clickable underneath it or
    // underneath an unrelated in-progress placement/drawing tool.
    let elements_selectable =
        is_editing && matches!(*tool.read(), Tool::Select) && !*picker_open.read();
    let gesture_label = gesture.read().label();

    // The currently selected camera (after preview), for the inspector.
    let selected_camera = selected_camera_id
        .as_ref()
        .and_then(|id| display_cameras.iter().find(|c| &c.camera_id == id).cloned());

    // The currently selected wall (after preview), for the inspector.
    let selected_wall = selected_wall_id
        .as_ref()
        .and_then(|id| display_walls.iter().find(|w| &w.id == id).cloned());

    // The currently selected door (after preview), for the inspector.
    let selected_door = selected_door_id
        .as_ref()
        .and_then(|id| display_doors.iter().find(|d| &d.id == id).cloned());

    // View-mode read-only info popover target. A tap pins a camera
    // (`info_camera_id`); hovering one sets `hovered_camera_id` (hover devices
    // only, gated in CSS). The pinned camera takes precedence, so the active id
    // is the pinned one when present, else the hovered one. `pinned` distinguishes
    // the two cases: a pinned popover gets a close button and is always shown,
    // while a hover-only popover is gated to hover devices and needs no close.
    let pinned_id = info_camera_id.read().clone();
    let pinned = pinned_id.is_some();
    let active_info_id = pinned_id
        .clone()
        .or_else(|| hovered_camera_id.read().clone());

    // Resolve the active id against the placed cameras (for its world position /
    // on-screen anchor) and the camera list (for its display data). A placed
    // reference with no matching camera is an orphan → `None` camera, shown as
    // "Unknown camera". The popover anchors to the marker's current on-screen
    // point, derived from the live viewport + canvas origin so it follows the
    // marker as the user pans/zooms.
    let info_anchor = active_info_id.as_ref().and_then(|id| {
        let placed_camera = display_cameras.iter().find(|c| &c.camera_id == id)?;
        let vp = *viewport.read();
        let (ox, oy) = *canvas_origin.read();
        let screen_x = ox + (placed_camera.position.x as f64 * vp.zoom + vp.pan_x);
        let screen_y = oy + (placed_camera.position.y as f64 * vp.zoom + vp.pan_y);
        let camera = camera_list.iter().find(|c| &c.id == id).cloned();
        Some((screen_x, screen_y, camera))
    });

    // --- Minimap inputs ---
    // The minimap only renders when there is content to navigate AND the canvas
    // has been measured (non-zero size). The outer box is the content bounds and
    // stays fixed as the user pans (its scale only changes when the content
    // itself does); `visible` is the world rect currently on screen, derived
    // from the viewport + canvas size, and may extend beyond the box. Auto-hide:
    // skip it when fully zoomed out (the visible rect already contains all the
    // content, so there is nothing off-screen to navigate to).
    let (canvas_w, canvas_h) = *canvas_size.read();
    let minimap_data = if canvas_w > 0.0 && canvas_h > 0.0 {
        content_bounds(&display_cameras, &display_walls, &display_doors).and_then(|content| {
            let vp = *viewport.read();
            let (vmin_x, vmin_y) = vp.screen_to_world(0.0, 0.0);
            let (vmax_x, vmax_y) = vp.screen_to_world(canvas_w, canvas_h);
            let visible = (vmin_x, vmin_y, vmax_x, vmax_y);
            if fully_contains_bounds(visible, content) {
                None
            } else {
                Some((content, visible))
            }
        })
    } else {
        None
    };

    // Zoom shown as a percentage of the auto-fit scale: 100% is the default
    // fit-to-content framing the map opens at, >100% is zoomed in past it. Falls
    // back to the raw scale when there is no content / unmeasured canvas to
    // define a fit reference.
    let zoom_percent = {
        let zoom = viewport.read().zoom;
        let fit_zoom = if canvas_w > 0.0 && canvas_h > 0.0 {
            content_bounds(&display_cameras, &display_walls, &display_doors)
                .map(|bounds| Viewport::fit_to_content(bounds, canvas_w, canvas_h).zoom)
        } else {
            None
        };
        match fit_zoom {
            Some(fit) if fit > 0.0 => (zoom / fit * 100.0).round() as i64,
            _ => (zoom * 100.0).round() as i64,
        }
    };

    rsx! {
        div { class: style::container,
            // --- Top bar (title, undo/redo, edit toggle) ---
            // Rendered before the canvas so it sits in normal flow above it.
            // Undo/redo (edit mode only) go in the start zone; the Edit/Done
            // toggle goes in the actions zone. The shared TopBar centers the
            // title regardless of the side controls.
            TopBar {
                title: if map_loading || cameras_loading { "Loading map…" } else { "Map" },
                start: rsx! {
                    if is_editing {
                        button {
                            class: style::topbar_icon,
                            disabled: !can_undo,
                            onclick: move |_| undo(()),
                            Icon { width: 18, height: 18, icon: LdCornerUpLeft }
                        }
                        button {
                            class: style::topbar_icon,
                            disabled: !can_redo,
                            onclick: move |_| redo(()),
                            Icon { width: 18, height: 18, icon: LdCornerUpRight }
                        }
                    }
                },
                actions: rsx! {
                    button {
                        class: style::topbar_edit,
                        "data-active": is_editing,
                        onclick: move |_| {
                            let next = !*editing.read();
                            editing.set(next);
                            if next {
                                info_camera_id.set(None);
                            } else {
                                selection.set(None);
                                tool.set(Tool::Select);
                                picker_open.set(false);
                            }
                        },
                        if is_editing {
                            "Done"
                        } else {
                            "Edit"
                        }
                    }
                },
            }

            // Frame is the svg's positioning context and the measured element:
            // the svg fills it via absolute inset:0, so the frame's rect is the
            // canvas rect. We measure the div (reliable, via the ResizeObserver
            // keyed on this id) rather than the svg, whose percentage size
            // collapses to its intrinsic 300x150 on WebKit. Because the svg
            // exactly overlaps the frame, the frame origin is the svg origin, so
            // `canvas_xy` pointer math stays correct.
            div {
                id: style::canvas_frame,
                class: style::canvas_frame,
                svg {
                    class: style::canvas,
                    "data-placing": is_placing,
                    "data-drawing-wall": is_drawing_wall,
                    "data-gesture": gesture_label,
                    xmlns: "http://www.w3.org/2000/svg",
                    // Touch-action none lets us own panning/pinching instead of
                    // the browser scrolling/zooming the page.
                    style: "touch-action: none;",

                // --- Wheel zoom (desktop) ---
                onwheel: move |evt| {
                    let delta = evt.data().delta().strip_units().y;
                    let client = evt.data().client_coordinates();
                    let origin = *canvas_origin.read();
                    let (sx, sy) = (client.x - origin.0, client.y - origin.1);
                    let factor = (-delta * WHEEL_ZOOM_STEP).exp();
                    viewport.write().zoom_at(factor, sx, sy);
                },

                // --- Pointer (mouse / single finger) ---
                // A pointer-down that reaches the canvas started on empty space
                // (marker / handle handlers stop propagation), so this is a pan.
                // In placing mode it instead drops the chosen camera; in edit
                // mode with nothing under it, it deselects.
                onpointerdown: move |evt| {
                    let (cx, cy) = canvas_xy(&evt.data(), *canvas_origin.read());
                    let pending = tool.read().clone();
                    if let Tool::PlaceCamera(camera_id) = pending {
                        let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                        place_camera(MapCamera {
                            camera_id: camera_id.clone(),
                            position: Point { x: wx.round() as i32, y: wy.round() as i32 },
                            fov: DEFAULT_FOV,
                        });
                        tool.set(Tool::Select);
                        selection.set(Some(Selection::Camera(camera_id)));
                        return;
                    }
                    if let Tool::DrawWall { mut vertices } = pending {
                        let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                        let world_point = Point { x: wx.round() as i32, y: wy.round() as i32 };

                        // Close-loop hit-test: only meaningful once there's an
                        // actual loop to close (need >= 3 vertices before
                        // "closing" makes geometric sense — with fewer points
                        // it would just double back on itself).
                        if vertices.len() >= 3 {
                            let (v0_sx, v0_sy) = viewport
                                .read()
                                .world_to_screen(vertices[0].x as f64, vertices[0].y as f64);
                            if distance(cx, cy, v0_sx, v0_sy) <= CLOSE_LOOP_HIT_RADIUS_PX {
                                place_wall(MapWall {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    vertices,
                                    closed: true,
                                    color: WallColor::default(),
                                });
                                tool.set(Tool::Select);
                                return;
                            }
                        }

                        vertices.push(world_point);
                        tool.set(Tool::DrawWall { vertices });
                        return;
                    }
                    if let Tool::PlaceDoor { start } = pending {
                        let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                        let world_point = Point { x: wx.round() as i32, y: wy.round() as i32 };
                        match start {
                            None => {
                                tool.set(Tool::PlaceDoor { start: Some(world_point) });
                            }
                            Some(start_point) => {
                                // A door is always exactly two points, so the
                                // second click both finishes AND commits in one
                                // step — unlike wall drafting there is no
                                // separate "finish" affordance. The newly
                                // placed door is deliberately left unselected,
                                // same as a newly-drawn wall: selecting it
                                // requires a follow-up tap on the opening line.
                                place_door(MapDoor {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    start: start_point,
                                    end: world_point,
                                    swing: DoorSwing::default(),
                                });
                                tool.set(Tool::Select);
                            }
                        }
                        return;
                    }
                    if is_editing {
                        selection.set(None);
                    } else {
                        // A press on empty canvas dismisses the view-mode info
                        // card. The marker stops propagation, so this only fires
                        // for taps that miss every camera.
                        info_camera_id.set(None);
                    }
                    gesture.set(Gesture::Pan { last_x: cx, last_y: cy });
                },
                onpointermove: move |evt| {
                    let (cx, cy) = canvas_xy(&evt.data(), *canvas_origin.read());
                    cursor_pos.set(Some((cx, cy)));
                    let current = gesture.read().clone();
                    match current {
                        Gesture::Pan { last_x, last_y } => {
                            viewport.write().pan_by(cx - last_x, cy - last_y);
                            gesture.set(Gesture::Pan { last_x: cx, last_y: cy });
                        }
                        Gesture::MoveCamera { camera_id, last_x, last_y } => {
                            let zoom = viewport.read().zoom;
                            let base = drag_preview
                                .read()
                                .position_for(&camera_id)
                                .or_else(|| {
                                    placed
                                        .iter()
                                        .find(|c| c.camera_id == camera_id)
                                        .map(|c| c.position.clone())
                                });
                            if let Some(base) = base {
                                let position = apply_drag_delta(base, cx, cy, last_x, last_y, zoom);
                                drag_preview
                                    .set(DragPreview::Position {
                                        camera_id: camera_id.clone(),
                                        position,
                                    });
                                gesture
                                    .set(Gesture::MoveCamera {
                                        camera_id,
                                        last_x: cx,
                                        last_y: cy,
                                    });
                            }
                        }
                        Gesture::AimCamera { camera_id } => {
                            if let Some(camera) = placed.iter().find(|c| c.camera_id == camera_id) {
                                let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                                let direction_deg = bearing_to(
                                    camera.position.x as f64,
                                    camera.position.y as f64,
                                    wx,
                                    wy,
                                );
                                let fov = FieldOfView {
                                    direction_deg,
                                    ..drag_preview.read().fov_for(&camera_id).unwrap_or(camera.fov.clone())
                                };
                                drag_preview.set(DragPreview::Fov { camera_id, fov });
                            }
                        }
                        Gesture::RangeCamera { camera_id } => {
                            if let Some(camera) = placed.iter().find(|c| c.camera_id == camera_id) {
                                let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                                let dist = distance(
                                    camera.position.x as f64,
                                    camera.position.y as f64,
                                    wx,
                                    wy,
                                );
                                let range = (dist.round() as i32).max(MIN_RANGE_CM);
                                let fov = FieldOfView {
                                    range,
                                    ..drag_preview.read().fov_for(&camera_id).unwrap_or(camera.fov.clone())
                                };
                                drag_preview.set(DragPreview::Fov { camera_id, fov });
                            }
                        }
                        Gesture::MoveWallVertex { wall_id, vertex_index, last_x, last_y } => {
                            let zoom = viewport.read().zoom;
                            let base = drag_preview
                                .read()
                                .wall_vertex_for(&wall_id, vertex_index)
                                .or_else(|| {
                                    placed_walls
                                        .iter()
                                        .find(|w| w.id == wall_id)
                                        .and_then(|w| w.vertices.get(vertex_index).cloned())
                                });
                            if let Some(base) = base {
                                let position = apply_drag_delta(base, cx, cy, last_x, last_y, zoom);
                                drag_preview
                                    .set(DragPreview::WallVertex {
                                        wall_id: wall_id.clone(),
                                        vertex_index,
                                        position,
                                    });
                                gesture
                                    .set(Gesture::MoveWallVertex {
                                        wall_id,
                                        vertex_index,
                                        last_x: cx,
                                        last_y: cy,
                                    });
                            }
                        }
                        Gesture::MoveDoorEndpoint { door_id, which, last_x, last_y } => {
                            let zoom = viewport.read().zoom;
                            let base = drag_preview
                                .read()
                                .door_endpoint_for(&door_id, which)
                                .or_else(|| {
                                    placed_doors
                                        .iter()
                                        .find(|d| d.id == door_id)
                                        .map(|d| match which {
                                            Endpoint::Start => d.start.clone(),
                                            Endpoint::End => d.end.clone(),
                                        })
                                });
                            if let Some(base) = base {
                                let position = apply_drag_delta(base, cx, cy, last_x, last_y, zoom);
                                drag_preview
                                    .set(DragPreview::DoorEndpoint {
                                        door_id: door_id.clone(),
                                        which,
                                        position,
                                    });
                                gesture
                                    .set(Gesture::MoveDoorEndpoint {
                                        door_id,
                                        which,
                                        last_x: cx,
                                        last_y: cy,
                                    });
                            }
                        }
                        Gesture::None | Gesture::Pinch { .. } => {}
                    }
                },
                onpointerup: move |_| {
                    // Commit exactly one edit for the gesture that just ended.
                    let current = gesture.read().clone();
                    match current {
                        Gesture::MoveCamera { camera_id, .. } => {
                            if let DragPreview::Position { position, .. } = drag_preview.read().clone() {
                                move_camera((camera_id, position));
                            }
                        }
                        Gesture::AimCamera { camera_id } | Gesture::RangeCamera { camera_id } => {
                            if let DragPreview::Fov { fov, .. } = drag_preview.read().clone() {
                                aim_camera((camera_id, fov));
                            }
                        }
                        Gesture::MoveWallVertex { wall_id, vertex_index, .. } => {
                            if let DragPreview::WallVertex { position, .. } = drag_preview.read().clone() {
                                move_wall_vertex((wall_id, vertex_index, position));
                            }
                        }
                        Gesture::MoveDoorEndpoint { door_id, which, .. } => {
                            if let DragPreview::DoorEndpoint { position, .. } = drag_preview.read().clone() {
                                move_door_endpoint((door_id, which == Endpoint::Start, position));
                            }
                        }
                        _ => {}
                    }
                    drag_preview.set(DragPreview::None);
                    gesture.set(Gesture::None);
                },
                onpointercancel: move |_| {
                    drag_preview.set(DragPreview::None);
                    gesture.set(Gesture::None);
                },
                onpointerleave: move |_| {
                    cursor_pos.set(None);
                },

                // --- Finish an open wall path (double-click) ---
                ondoubleclick: move |_| {
                    let Tool::DrawWall { mut vertices } = tool.read().clone() else { return; };

                    // A double-click is physically two separate clicks in quick
                    // succession, both of which already ran through
                    // onpointerdown above and each pushed a vertex at
                    // (approximately) the same point. Drop that trailing
                    // duplicate so finishing a path doesn't leave a spurious
                    // extra vertex at the exact spot the user double-clicked.
                    if vertices.len() >= 2 {
                        let last = vertices.len() - 1;
                        // Screen-space, not world-space: both points came from
                        // clicks at essentially the same physical pixel, but
                        // that maps to wildly different world-cm distances
                        // depending on zoom (e.g. 1px is 50 world-cm at
                        // MIN_ZOOM), so a fixed world-space threshold either
                        // over- or under-fires depending on zoom level.
                        let (last_sx, last_sy) = viewport
                            .read()
                            .world_to_screen(vertices[last].x as f64, vertices[last].y as f64);
                        let (prev_sx, prev_sy) = viewport.read().world_to_screen(
                            vertices[last - 1].x as f64,
                            vertices[last - 1].y as f64,
                        );
                        let d = distance(last_sx, last_sy, prev_sx, prev_sy);
                        if d < DOUBLE_CLICK_DEDUP_RADIUS_PX {
                            vertices.pop();
                        }
                    }

                    if vertices.len() >= 2 {
                        place_wall(MapWall {
                            id: uuid::Uuid::new_v4().to_string(),
                            vertices,
                            closed: false,
                            color: WallColor::default(),
                        });
                    }
                    tool.set(Tool::Select);
                },

                // --- Touch pinch zoom ---
                ontouchstart: move |evt| {
                    let touches = evt.data().touches();
                    if touches.len() == 2 {
                        let a = touches[0].client_coordinates();
                        let b = touches[1].client_coordinates();
                        gesture
                            .set(Gesture::Pinch {
                                last_distance: distance(a.x, a.y, b.x, b.y),
                            });
                    }
                },
                ontouchmove: move |evt| {
                    let touches = evt.data().touches();
                    let current = gesture.read().clone();
                    if touches.len() == 2
                        && let Gesture::Pinch { last_distance, .. } = current
                    {
                        let a = touches[0].client_coordinates();
                        let b = touches[1].client_coordinates();
                        let dist = distance(a.x, a.y, b.x, b.y);
                        let cx = (a.x + b.x) / 2.0;
                        let cy = (a.y + b.y) / 2.0;
                        if last_distance > 0.0 {
                            let factor = dist / last_distance;
                            viewport.write().zoom_at(factor, cx, cy);
                        }
                        gesture.set(Gesture::Pinch { last_distance: dist });
                    }
                },
                ontouchend: move |_| {
                    if matches!(*gesture.read(), Gesture::Pinch { .. }) {
                        gesture.set(Gesture::None);
                    }
                },

                // Faint grid backdrop, stepped through "nice" 1-2-5 world-space
                // spacings (see `nice_step_at_least`) so squares stay legible
                // instead of shrinking to mush when zoomed out or ballooning when
                // zoomed in. The pattern tile is defined in world (cm) units and
                // carries the same pan/zoom transform as the content group below,
                // so grid lines stay locked to whole world coordinates as the
                // user pans/zooms. `vector-effect: non-scaling-stroke` keeps the
                // line itself a constant 1 screen pixel regardless of that
                // transform's scale, so it stays visible at every zoom level.
                defs {
                    pattern {
                        id: "map-grid",
                        width: "{grid_spacing_cm}",
                        height: "{grid_spacing_cm}",
                        "patternUnits": "userSpaceOnUse",
                        "patternTransform": "{transform}",
                        path {
                            d: "M {grid_spacing_cm} 0 L 0 0 0 {grid_spacing_cm}",
                            fill: "none",
                            stroke: "#2a2f3e",
                            "stroke-width": "1",
                            "vector-effect": "non-scaling-stroke",
                        }
                    }
                }
                rect { width: "100%", height: "100%", fill: "url(#map-grid)" }

                // World-space content: pan + zoom applied here so cameras stay in
                // logical cm coordinates.
                g { transform: "{transform}",
                    for camera in display_cameras.iter().cloned() {
                        {
                            let id = camera.camera_id.clone();
                            let orphaned = name_for(&id).is_none();
                            let is_selected = selected_camera_id.as_deref() == Some(id.as_str());
                            rsx! {
                                MapCameraMarker {
                                    key: "{id}",
                                    camera,
                                    selected: is_selected,
                                    editing: is_editing,
                                    interactive: elements_selectable,
                                    orphaned,
                                    on_body_pointer_down: {
                                        let id = id.clone();
                                        move |evt: Event<PointerData>| {
                                            selection.set(Some(Selection::Camera(id.clone())));
                                            let (cx, cy) = canvas_xy(&evt.data(), *canvas_origin.read());
                                            gesture
                                                .set(Gesture::MoveCamera {
                                                    camera_id: id.clone(),
                                                    last_x: cx,
                                                    last_y: cy,
                                                });
                                        }
                                    },
                                    on_aim_pointer_down: {
                                        let id = id.clone();
                                        move |_evt: Event<PointerData>| {
                                            gesture.set(Gesture::AimCamera { camera_id: id.clone() });
                                        }
                                    },
                                    on_range_pointer_down: {
                                        let id = id.clone();
                                        move |_evt: Event<PointerData>| {
                                            gesture.set(Gesture::RangeCamera { camera_id: id.clone() });
                                        }
                                    },
                                    on_tap: {
                                        // View mode only: pin the read-only info
                                        // popover for this camera. In edit mode the
                                        // pointer-down selection flow owns taps, so
                                        // ignore this here.
                                        let id = id.clone();
                                        move |_| {
                                            if !is_editing {
                                                info_camera_id.set(Some(id.clone()));
                                            }
                                        }
                                    },
                                    on_hover_enter: {
                                        // Hover-capable devices only (gated in CSS):
                                        // show the popover for the hovered camera.
                                        // The pinned camera takes precedence when set.
                                        let id = id.clone();
                                        move |_| {
                                            if !is_editing {
                                                hovered_camera_id.set(Some(id.clone()));
                                            }
                                        }
                                    },
                                    on_hover_leave: {
                                        let id = id.clone();
                                        move |_| {
                                            if hovered_camera_id.read().as_deref()
                                                == Some(id.as_str())
                                            {
                                                hovered_camera_id.set(None);
                                            }
                                        }
                                    },
                                }
                            }
                        }
                    }

                    for wall in display_walls.iter().cloned() {
                        {
                            let id = wall.id.clone();
                            let is_selected = selected_wall_id.as_deref() == Some(id.as_str());
                            rsx! {
                                MapWallPath {
                                    key: "{id}",
                                    wall,
                                    selected: is_selected,
                                    interactive: elements_selectable,
                                    on_path_pointer_down: {
                                        let id = id.clone();
                                        move |_evt: Event<PointerData>| {
                                            selection.set(Some(Selection::Wall(id.clone())));
                                        }
                                    },
                                    on_vertex_pointer_down: {
                                        let id = id.clone();
                                        move |(vertex_index, evt): (usize, Event<PointerData>)| {
                                            let (cx, cy) = canvas_xy(&evt.data(), *canvas_origin.read());
                                            gesture.set(Gesture::MoveWallVertex {
                                                wall_id: id.clone(),
                                                vertex_index,
                                                last_x: cx,
                                                last_y: cy,
                                            });
                                        }
                                    },
                                }
                            }
                        }
                    }

                    for door in display_doors.iter().cloned() {
                        {
                            let id = door.id.clone();
                            let is_selected = selected_door_id.as_deref() == Some(id.as_str());
                            rsx! {
                                MapDoorMarker {
                                    key: "{id}",
                                    door,
                                    selected: is_selected,
                                    interactive: elements_selectable,
                                    on_body_pointer_down: {
                                        let id = id.clone();
                                        move |_evt: Event<PointerData>| {
                                            selection.set(Some(Selection::Door(id.clone())));
                                        }
                                    },
                                    on_endpoint_pointer_down: {
                                        let id = id.clone();
                                        move |(which, evt): (Endpoint, Event<PointerData>)| {
                                            let (cx, cy) = canvas_xy(&evt.data(), *canvas_origin.read());
                                            gesture.set(Gesture::MoveDoorEndpoint {
                                                door_id: id.clone(),
                                                which,
                                                last_x: cx,
                                                last_y: cy,
                                            });
                                        }
                                    },
                                }
                            }
                        }
                    }

                    // --- In-progress door placement preview ---
                    // A rubber-band line from the already-placed start point to
                    // the live cursor position while the second click is still
                    // pending, same technique the wall draft's rubber-band
                    // uses. Reuses `.draft_rubber_band` directly
                    // (same visual language: "a tentative, not-yet-committed
                    // line") rather than a near-duplicate class.
                    if let Tool::PlaceDoor { start: Some(point) } = &*tool.read()
                        && let Some((cx, cy)) = *cursor_pos.read()
                    {
                        {
                            let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                            rsx! {
                                line {
                                    class: style::draft_rubber_band,
                                    x1: "{point.x}",
                                    y1: "{point.y}",
                                    x2: "{wx}",
                                    y2: "{wy}",
                                }
                            }
                        }
                    }

                    // --- In-progress wall draft ---
                    // Rendered here (inside the world-space group) so it pans
                    // and zooms with everything else. No selection/editing of
                    // finished walls yet — this is purely the live-drawing
                    // preview for the active `Tool::DrawWall` draft.
                    if let Tool::DrawWall { vertices } = &*tool.read() {
                        // Committed segments so far: a plain open polyline,
                        // never closed while still drafting.
                        if vertices.len() >= 2 {
                            {
                                let mut parts = Vec::with_capacity(vertices.len());
                                for (i, v) in vertices.iter().enumerate() {
                                    let cmd = if i == 0 { "M" } else { "L" };
                                    parts.push(format!("{cmd} {} {}", v.x, v.y));
                                }
                                let d = parts.join(" ");
                                rsx! {
                                    path { class: style::draft_path, d: "{d}" }
                                }
                            }
                        }

                        // Rubber-band segment from the last committed vertex to
                        // the live cursor position, derived from `cursor_pos`
                        // (reused rather than tracked separately).
                        if let (Some(last), Some((cx, cy))) = (vertices.last(), *cursor_pos.read())
                        {
                            {
                                let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                                rsx! {
                                    line {
                                        class: style::draft_rubber_band,
                                        x1: "{last.x}",
                                        y1: "{last.y}",
                                        x2: "{wx}",
                                        y2: "{wy}",
                                    }
                                }
                            }
                        }

                        // A small dot at each committed vertex.
                        for (i , v) in vertices.iter().enumerate() {
                            circle {
                                key: "{i}",
                                class: style::draft_vertex,
                                cx: "{v.x}",
                                cy: "{v.y}",
                                r: "{MARKER_RADIUS_CM * 0.3}",
                            }
                        }

                        // Once there are enough vertices to close a loop,
                        // highlight the first vertex as the close-loop target,
                        // with a hover affordance once the cursor is actually
                        // within the auto-close hit radius (same threshold the
                        // pointerdown handler uses to commit the close).
                        if vertices.len() >= 3
                            && let Some(first) = vertices.first()
                        {
                            {
                                let in_range = cursor_pos.read().is_some_and(|(cx, cy)| {
                                    let (v0_sx, v0_sy) = viewport
                                        .read()
                                        .world_to_screen(first.x as f64, first.y as f64);
                                    distance(cx, cy, v0_sx, v0_sy) <= CLOSE_LOOP_HIT_RADIUS_PX
                                });
                                rsx! {
                                    circle {
                                        class: style::draft_close_target,
                                        "data-in-range": in_range,
                                        cx: "{first.x}",
                                        cy: "{first.y}",
                                        r: "{MARKER_RADIUS_CM}",
                                    }
                                }
                            }
                        }
                    }
                }
                }

                // --- Coordinate readout (placement tools and vertex drags) ---
                // Follows the pointer, offset slightly so the label doesn't sit
                // directly under the cursor/finger. Shown while a placement
                // tool is armed, and while dragging an existing camera, wall
                // vertex, or door endpoint (using the same previewed position
                // the canvas is rendering, not a fresh screen-to-world lookup,
                // so the readout always matches what's on screen).
                if let Some((cx, cy)) = *cursor_pos.read() {
                    {
                        let coords = drag_preview.read().dragged_vertex_position()
                            .map(|position| (position.x, position.y))
                            .or_else(|| {
                                if matches!(*tool.read(), Tool::Select) {
                                    None
                                } else {
                                    let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                                    Some((wx.round() as i32, wy.round() as i32))
                                }
                            });
                        rsx! {
                            if let Some((wx, wy)) = coords {
                                div {
                                    class: style::coord_readout,
                                    style: "left: {cx + 14.0}px; top: {cy + 14.0}px;",
                                    "{wx}, {wy} cm",
                                }
                            }
                        }
                    }
                }
            }

            // --- Bottom chrome (edit mode only) ---
            // Inspector takes precedence over the tool strip when a camera,
            // wall, or door is selected (camera first, then wall, then door).
            // Both sit above the global navigation toolbar.
            if is_editing {
                if let Some(camera) = selected_camera.clone() {
                    CameraInspector {
                        name: name_for(&camera.camera_id),
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
                } else if let Some(wall) = selected_wall.clone() {
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
                } else if let Some(door) = selected_door.clone() {
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

            // --- View-mode read-only info popover ---
            // Anchored next to the marker (tapped, or hovered on hover devices)
            // outside edit mode. It follows the marker on pan/zoom because the
            // anchor is recomputed from the live viewport each render. A pinned
            // (tapped) popover shows a close button and is always rendered; a
            // hover-only one is gated to hover devices in CSS and needs no close.
            if !is_editing {
                if let Some((screen_x, screen_y, camera)) = info_anchor {
                    CameraInfo {
                        camera,
                        anchor_x: screen_x,
                        anchor_y: screen_y,
                        pinned,
                        on_close: move |_| info_camera_id.set(None),
                    }
                }
            }

            // --- Minimap (bottom-right viewport navigator) ---
            // Auto-hidden when fully zoomed out (see `minimap_data`). Recentering
            // keeps zoom and pans so the chosen world point maps to the canvas
            // center. A tall bottom sheet (inspector or picker), when open, renders
            // over the minimap via z-index rather than displacing it.
            if let Some((world_bounds, visible)) = minimap_data {
                Minimap {
                    world_bounds,
                    visible,
                    // Live positions (preview applied) so dots track during drags.
                    cameras: display_cameras
                        .iter()
                        .map(|c| (c.position.x as f64, c.position.y as f64))
                        .collect::<Vec<_>>(),
                    on_recenter: move |(wx, wy): (f64, f64)| {
                        let (cw, ch) = *canvas_size.read();
                        let mut vp = viewport.write();
                        let zoom = vp.zoom;
                        vp.pan_x = cw / 2.0 - wx * zoom;
                        vp.pan_y = ch / 2.0 - wy * zoom;
                    },
                }
            }

            // --- Unplaced-camera badge (lower-left) ---
            // A floating warning that some cameras exist but are not yet on the
            // map, shown in both view and edit modes. Hidden when everything is
            // placed (count == 0). Its z-index sits below the bottom sheets, so an
            // open inspector / picker simply renders over it.
            if !unplaced.is_empty() {
                UnplacedBadge { count: unplaced.len() }
            }

            // --- Zoom controls (persistent, below the minimap) ---
            // Always rendered, even when the minimap auto-hides: zooming in with
            // `+` can bring the hidden minimap back. The `+` / `−` buttons zoom
            // around the canvas center (so the center stays put), clamped by the
            // existing zoom clamp.
            ZoomControls {
                percent: zoom_percent,
                on_zoom_out: move |_| {
                    let (cw, ch) = *canvas_size.read();
                    viewport.write().zoom_at(1.0 / BUTTON_ZOOM_STEP, cw / 2.0, ch / 2.0);
                },
                on_zoom_in: move |_| {
                    let (cw, ch) = *canvas_size.read();
                    viewport.write().zoom_at(BUTTON_ZOOM_STEP, cw / 2.0, ch / 2.0);
                },
                on_reset_zoom: move |_| {
                    let (cw, ch) = *canvas_size.read();
                    if let Some(bounds) = content_bounds(&display_cameras, &display_walls, &display_doors) {
                        viewport.set(Viewport::fit_to_content(bounds, cw, ch));
                    }
                },
            }

            // --- Camera picker sheet ---
            if is_editing && *picker_open.read() {
                CameraPicker {
                    cameras: unplaced.clone(),
                    on_pick: move |id: String| {
                        tool.set(Tool::PlaceCamera(id));
                        picker_open.set(false);
                    },
                    on_close: move |_| picker_open.set(false),
                }
            }
        }
    }
}

impl DragPreview {
    /// The previewed position for `camera_id`, if a position preview is active.
    fn position_for(&self, camera_id: &str) -> Option<Point> {
        match self {
            DragPreview::Position {
                camera_id: id,
                position,
            } if id == camera_id => Some(position.clone()),
            _ => None,
        }
    }

    /// The previewed FOV for `camera_id`, if a FOV preview is active.
    fn fov_for(&self, camera_id: &str) -> Option<FieldOfView> {
        match self {
            DragPreview::Fov { camera_id: id, fov } if id == camera_id => Some(fov.clone()),
            _ => None,
        }
    }

    /// The previewed position for vertex `vertex_index` of `wall_id`, if a
    /// matching wall-vertex preview is active.
    fn wall_vertex_for(&self, wall_id: &str, vertex_index: usize) -> Option<Point> {
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
    fn door_endpoint_for(&self, door_id: &str, which: Endpoint) -> Option<Point> {
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
    fn dragged_vertex_position(&self) -> Option<Point> {
        match self {
            DragPreview::Position { position, .. }
            | DragPreview::WallVertex { position, .. }
            | DragPreview::DoorEndpoint { position, .. } => Some(position.clone()),
            DragPreview::Fov { .. } | DragPreview::None => None,
        }
    }
}
