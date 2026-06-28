use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::fi_icons::{FiCornerUpLeft, FiCornerUpRight};
use shield_models::{FieldOfView, MapCamera, Point};

use crate::components::map::camera_inspector::CameraInspector;
use crate::components::map::edit_toolbar::{CameraPicker, EditToolbar};
use crate::components::map::map_camera::MapCameraMarker;
use crate::hooks::{UseCamerasResult, UseMapResult, use_cameras, use_map};

/// The single map edited in v1. The service lazily returns an empty map for any
/// id, so a fixed default is sufficient until multi-map UI exists.
const DEFAULT_MAP_ID: &str = "default";

/// Default field-of-view applied to a freshly placed camera.
const DEFAULT_FOV: FieldOfView = FieldOfView {
    direction_deg: 0,
    angle_deg: 70,
    range: 500,
};

/// Minimum / maximum zoom (screen px per logical cm). Panning is intentionally
/// unconstrained, but zoom is clamped to keep the canvas usable.
const MIN_ZOOM: f64 = 0.02;
const MAX_ZOOM: f64 = 5.0;

/// Multiplier applied per wheel "click" / pinch step.
const WHEEL_ZOOM_STEP: f64 = 0.0015;

/// Smallest range a camera cone may be dragged to (centimeters).
const MIN_RANGE_CM: i32 = 50;

/// Viewport transform mapping logical world coordinates (centimeters) to screen
/// pixels: `screen = world * zoom + pan`.
///
/// This is the single source of truth for what part of the map is on screen and
/// is deliberately small and self-describing so later rounds can build on it
/// directly — interactive manipulation needs screen->world for hit-testing/dragging,
/// and a minimap needs the world rect currently visible. Both can be derived from
/// these three fields plus the canvas size.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Viewport {
    /// Horizontal pan offset, in screen pixels.
    pan_x: f64,
    /// Vertical pan offset, in screen pixels.
    pan_y: f64,
    /// Scale factor: screen pixels per logical centimeter.
    zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        // Start zoomed out enough to comfortably show a ~2000x1500 area with the
        // origin near the top-left of the canvas.
        Self {
            pan_x: 40.0,
            pan_y: 40.0,
            zoom: 0.25,
        }
    }
}

impl Viewport {
    /// Pan by a screen-pixel delta (unconstrained).
    fn pan_by(&mut self, dx: f64, dy: f64) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Zoom by `factor` while keeping the screen point `(sx, sy)` anchored over
    /// the same world coordinate (zoom-to-cursor / zoom-to-pinch-center).
    fn zoom_at(&mut self, factor: f64, sx: f64, sy: f64) {
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // World point under the anchor before zooming.
        let wx = (sx - self.pan_x) / self.zoom;
        let wy = (sy - self.pan_y) / self.zoom;
        // Re-derive pan so that world point stays under the anchor.
        self.pan_x = sx - wx * new_zoom;
        self.pan_y = sy - wy * new_zoom;
        self.zoom = new_zoom;
    }

    /// Convert a screen-pixel point (relative to the canvas element) to a world
    /// coordinate in centimeters.
    fn screen_to_world(&self, sx: f64, sy: f64) -> (f64, f64) {
        ((sx - self.pan_x) / self.zoom, (sy - self.pan_y) / self.zoom)
    }

    /// SVG transform string for the world-space content group.
    fn transform(&self) -> String {
        format!(
            "translate({} {}) scale({})",
            self.pan_x, self.pan_y, self.zoom
        )
    }
}

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
        }
    }
}

/// A local, uncommitted preview of an in-progress manipulation. The canvas
/// renders from this instead of the stored map while a gesture is active so the
/// user sees live feedback; the matching edit is committed once on release.
#[derive(Clone, PartialEq)]
enum DragPreview {
    None,
    Position { camera_id: String, position: Point },
    Fov { camera_id: String, fov: FieldOfView },
}

/// Euclidean distance between two points.
fn distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

/// World-space bearing from a camera center to a world point, expressed as a
/// true-North clockwise direction in whole degrees (0 = up/North), matching the
/// FOV convention. Inverse of the cone math: screen angle `theta` (clockwise
/// from +x, y-down) relates to bearing `b` by `b = theta + 90`.
fn bearing_to(cx: f64, cy: f64, wx: f64, wy: f64) -> u16 {
    let theta = (wy - cy).atan2(wx - cx).to_degrees();
    let bearing = (theta + 90.0).rem_euclid(360.0);
    bearing.round() as u16
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
        undo,
        redo,
        can_undo,
        can_redo,
    } = use_map(DEFAULT_MAP_ID.to_string());

    let UseCamerasResult {
        cameras: camera_list,
        loading: cameras_loading,
    } = use_cameras();

    let mut viewport = use_signal(Viewport::default);
    let mut gesture = use_signal(|| Gesture::None);
    let mut drag_preview = use_signal(|| DragPreview::None);

    let mut editing = use_signal(|| false);
    let mut selection = use_signal(|| None::<String>);
    // The camera id chosen in the picker and awaiting a placement tap.
    let mut placing = use_signal(|| None::<String>);
    let mut picker_open = use_signal(|| false);

    // Cached viewport-relative top-left of the canvas. All pointer math is done
    // in canvas-relative coordinates derived from this origin (see `canvas_xy`).
    // The rect is stable during a drag, so the cached value stays correct; it is
    // captured on mount and refreshed on resize.
    let mut canvas_origin = use_signal(|| (0.0_f64, 0.0_f64));
    let mut canvas_element = use_signal(|| None::<std::rc::Rc<MountedData>>);

    let refresh_origin = use_callback(move |_: ()| {
        if let Some(element) = canvas_element.read().clone() {
            spawn(async move {
                if let Ok(rect) = element.get_client_rect().await {
                    canvas_origin.set((rect.origin.x, rect.origin.y));
                }
            });
        }
    });

    // Keep the cached canvas origin in sync with window resizes (the rect's
    // top-left can shift when the layout reflows).
    use_effect(move || {
        let mut resize = document::eval(
            "window.addEventListener('resize', () => dioxus.send(0)); dioxus.send(0);",
        );
        spawn(async move {
            while resize.recv::<i32>().await.is_ok() {
                refresh_origin(());
            }
        });
    });

    let placed = map.as_ref().map(|m| m.cameras.clone()).unwrap_or_default();

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

    let transform = viewport.read().transform();
    let selected_id = selection.read().clone();
    let is_editing = *editing.read();
    let is_placing = placing.read().is_some();
    let gesture_label = gesture.read().label();

    // The currently selected camera (after preview), for the inspector.
    let selected_camera = selected_id
        .as_ref()
        .and_then(|id| display_cameras.iter().find(|c| &c.camera_id == id).cloned());

    rsx! {
        div { class: "primary-view map-view",
            // --- Top bar (title, undo/redo, edit toggle) ---
            // Rendered before the canvas so it sits in normal flow above it.
            div { class: "map-topbar",
                if is_editing {
                    div { class: "map-topbar__history",
                        button {
                            class: "map-topbar__icon",
                            disabled: !can_undo,
                            onclick: move |_| undo(()),
                            Icon { width: 18, height: 18, icon: FiCornerUpLeft }
                        }
                        button {
                            class: "map-topbar__icon",
                            disabled: !can_redo,
                            onclick: move |_| redo(()),
                            Icon { width: 18, height: 18, icon: FiCornerUpRight }
                        }
                    }
                }

                span { class: "map-topbar__title",
                    if map_loading || cameras_loading {
                        "Loading map…"
                    } else {
                        "Map"
                    }
                }

                button {
                    class: "map-topbar__edit",
                    "data-active": is_editing,
                    onclick: move |_| {
                        let next = !*editing.read();
                        editing.set(next);
                        if !next {
                            selection.set(None);
                            placing.set(None);
                            picker_open.set(false);
                        }
                    },
                    if is_editing {
                        "Done"
                    } else {
                        "Edit"
                    }
                }
            }

            svg {
                class: "map-canvas",
                "data-placing": is_placing,
                "data-gesture": gesture_label,
                xmlns: "http://www.w3.org/2000/svg",
                // Touch-action none lets us own panning/pinching instead of the
                // browser scrolling/zooming the page.
                style: "touch-action: none;",

                onmounted: move |evt| {
                    canvas_element.set(Some(evt.data()));
                    refresh_origin(());
                },

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
                    let pending = placing.read().clone();
                    if let Some(camera_id) = pending {
                        let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                        place_camera(MapCamera {
                            camera_id: camera_id.clone(),
                            position: Point { x: wx.round() as i32, y: wy.round() as i32 },
                            fov: DEFAULT_FOV,
                        });
                        placing.set(None);
                        selection.set(Some(camera_id));
                        return;
                    }
                    if is_editing {
                        selection.set(None);
                    }
                    gesture.set(Gesture::Pan { last_x: cx, last_y: cy });
                },
                onpointermove: move |evt| {
                    let (cx, cy) = canvas_xy(&evt.data(), *canvas_origin.read());
                    let current = gesture.read().clone();
                    match current {
                        Gesture::Pan { last_x, last_y } => {
                            viewport.write().pan_by(cx - last_x, cy - last_y);
                            gesture.set(Gesture::Pan { last_x: cx, last_y: cy });
                        }
                        Gesture::MoveCamera { camera_id, last_x, last_y } => {
                            let zoom = viewport.read().zoom;
                            let dx = (cx - last_x) / zoom;
                            let dy = (cy - last_y) / zoom;
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
                                let position = Point {
                                    x: base.x + dx.round() as i32,
                                    y: base.y + dy.round() as i32,
                                };
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
                        _ => {}
                    }
                    drag_preview.set(DragPreview::None);
                    gesture.set(Gesture::None);
                },
                onpointercancel: move |_| {
                    drag_preview.set(DragPreview::None);
                    gesture.set(Gesture::None);
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

                // Faint grid backdrop (screen-fixed) to hint at the surface.
                defs {
                    pattern {
                        id: "map-grid",
                        width: "32",
                        height: "32",
                        "patternUnits": "userSpaceOnUse",
                        path {
                            d: "M 32 0 L 0 0 0 32",
                            fill: "none",
                            stroke: "#2a2f3e",
                            "stroke-width": "1",
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
                            let is_selected = selected_id.as_deref() == Some(id.as_str());
                            rsx! {
                                MapCameraMarker {
                                    key: "{id}",
                                    camera,
                                    selected: is_selected,
                                    editing: is_editing,
                                    orphaned,
                                    on_body_pointer_down: {
                                        let id = id.clone();
                                        move |evt: Event<PointerData>| {
                                            selection.set(Some(id.clone()));
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
                                }
                            }
                        }
                    }
                }
            }

            // --- Bottom chrome (edit mode only) ---
            // Inspector takes precedence over the tool strip when a camera is
            // selected. Both sit above the global navigation toolbar.
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
                } else {
                    EditToolbar {
                        on_add: move |_| {
                            placing.set(None);
                            picker_open.set(true);
                        },
                    }
                }
            }

            // --- Camera picker sheet ---
            if is_editing && *picker_open.read() {
                CameraPicker {
                    cameras: unplaced.clone(),
                    on_pick: move |id: String| {
                        placing.set(Some(id));
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
}
