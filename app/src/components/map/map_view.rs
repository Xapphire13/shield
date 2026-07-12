use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdCornerUpLeft, LdCornerUpRight};
use shield_models::{MapCamera, MapDoor, MapWall, WallColor};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::components::layout::TopBar;
use crate::components::map::camera_info::CameraInfo;
use crate::components::map::camera_inspector::CameraInspector;
use crate::components::map::canvas_gestures::{
    CLOSE_LOOP_HIT_RADIUS_PX, MapCommit, PointerMoveOutcome, ToolDownAction, canvas_xy,
    finish_wall_draft, pinch_move, pinch_start, pointer_move_transition, pointer_up_commit,
    tool_pointer_down,
};
use crate::components::map::door_inspector::DoorInspector;
use crate::components::map::edit_toolbar::{CameraPicker, EditToolbar};
use crate::components::map::geometry::{content_bounds, distance, fully_contains_bounds};
use crate::components::map::interaction::{
    DragPreview, EscapeAction, Gesture, Selection, Tool, escape_transition,
};
use crate::components::map::map_camera::{MARKER_RADIUS_CM, MapCameraMarker};
use crate::components::map::map_door::{Endpoint, MapDoorMarker};
use crate::components::map::map_wall::MapWallPath;
use crate::components::map::minimap::Minimap;
use crate::components::map::unplaced_badge::UnplacedBadge;
use crate::components::map::viewport::{BUTTON_ZOOM_STEP, Viewport, WHEEL_ZOOM_STEP};
use crate::components::map::wall_inspector::WallInspector;
use crate::components::map::zoom_controls::ZoomControls;
use crate::hooks::{
    UseCamerasResult, UseElementRectResult, UseMapResult, after_next_layout, element_rect,
    use_cameras, use_element_rect, use_map,
};

/// The single map edited in v1. The service lazily returns an empty map for any
/// id, so a fixed default is sufficient until multi-map UI exists.
const DEFAULT_MAP_ID: &str = "default";

/// DOM id of the canvas frame element, used to locate it for measurement.
const CANVAS_FRAME_ID: &str = style::canvas_frame;

stylance::import_crate_style!(style, "src/components/map/map_view.module.css");

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

    // Cached canvas geometry from the frame's bounding rect, tracked by a
    // ResizeObserver (see `use_element_rect`): the viewport-relative top-left
    // (origin) drives canvas-relative pointer math (see `canvas_xy`), and the
    // size drives the initial fit-to-content. The rect is stable during a
    // drag, so the cached values stay correct.
    let UseElementRectResult {
        origin: canvas_origin,
        size: canvas_size,
    } = use_element_rect(CANVAS_FRAME_ID);
    // Whether the initial fit-to-content has been applied. Guards against
    // re-fitting on later edits / pans / zooms.
    let mut fitted = use_signal(|| false);

    // Escape backs out the innermost active state (see `escape_transition`
    // for the cascade). Listened for at the document level; the closure and
    // listener registration are kept alive together in component state for
    // the component's lifetime.
    let _keydown_listener = use_hook(|| {
        let callback = Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
            if let Ok(evt) = evt.dyn_into::<web_sys::KeyboardEvent>()
                && evt.key() == "Escape"
            {
                let current = tool.read().clone();
                let picker = *picker_open.read();
                match escape_transition(&current, picker) {
                    EscapeAction::SetTool(next) => tool.set(next),
                    EscapeAction::ClosePicker => picker_open.set(false),
                    EscapeAction::ExitEditMode => {
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
                let Some((_, _, width, height)) = element_rect(CANVAS_FRAME_ID) else {
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

    // Apply any active drag preview so the canvas reflects the in-progress
    // gesture instead of the stored map.
    let preview = drag_preview.read().clone();
    let display_cameras: Vec<MapCamera> = preview.apply_to_cameras(&placed);
    let display_walls: Vec<MapWall> = preview.apply_to_walls(&placed_walls);
    let display_doors: Vec<MapDoor> = preview.apply_to_doors(&placed_doors);

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
                    if let Some(action) = tool_pointer_down(pending, *viewport.read(), cx, cy) {
                        match action {
                            ToolDownAction::PlaceCamera(camera) => {
                                let camera_id = camera.camera_id.clone();
                                place_camera(camera);
                                tool.set(Tool::Select);
                                selection.set(Some(Selection::Camera(camera_id)));
                            }
                            ToolDownAction::CloseWallLoop(wall) => {
                                place_wall(wall);
                                tool.set(Tool::Select);
                            }
                            ToolDownAction::ExtendWallDraft(vertices) => {
                                tool.set(Tool::DrawWall { vertices });
                            }
                            ToolDownAction::SetDoorStart(start) => {
                                tool.set(Tool::PlaceDoor { start: Some(start) });
                            }
                            ToolDownAction::PlaceDoor(door) => {
                                place_door(door);
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
                    let outcome = pointer_move_transition(
                        &gesture.read().clone(),
                        &drag_preview.read().clone(),
                        *viewport.read(),
                        &placed,
                        &placed_walls,
                        &placed_doors,
                        cx,
                        cy,
                    );
                    match outcome {
                        PointerMoveOutcome::None => {}
                        PointerMoveOutcome::Pan { dx, dy, next } => {
                            viewport.write().pan_by(dx, dy);
                            gesture.set(next);
                        }
                        PointerMoveOutcome::Preview { preview, next } => {
                            drag_preview.set(preview);
                            if let Some(next) = next {
                                gesture.set(next);
                            }
                        }
                    }
                },
                onpointerup: move |_| {
                    // Commit exactly one edit for the gesture that just ended.
                    if let Some(commit) =
                        pointer_up_commit(&gesture.read().clone(), &drag_preview.read().clone())
                    {
                        match commit {
                            MapCommit::MoveCamera { camera_id, position } => {
                                move_camera((camera_id, position));
                            }
                            MapCommit::AimCamera { camera_id, fov } => {
                                aim_camera((camera_id, fov));
                            }
                            MapCommit::MoveWallVertex { wall_id, vertex_index, position } => {
                                move_wall_vertex((wall_id, vertex_index, position));
                            }
                            MapCommit::MoveDoorEndpoint { door_id, start, position } => {
                                move_door_endpoint((door_id, start, position));
                            }
                        }
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
                // See `finish_wall_draft` for the trailing-vertex dedup.
                ondoubleclick: move |_| {
                    let Tool::DrawWall { vertices } = tool.read().clone() else { return; };
                    if let Some(wall) = finish_wall_draft(vertices, *viewport.read()) {
                        place_wall(wall);
                    }
                    tool.set(Tool::Select);
                },

                // --- Touch pinch zoom ---
                ontouchstart: move |evt| {
                    let touches = evt.data().touches();
                    if touches.len() == 2 {
                        let a = touches[0].client_coordinates();
                        let b = touches[1].client_coordinates();
                        gesture.set(pinch_start((a.x, a.y), (b.x, b.y)));
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
                        let update = pinch_move(last_distance, (a.x, a.y), (b.x, b.y));
                        if let Some(factor) = update.factor {
                            viewport.write().zoom_at(factor, update.anchor.0, update.anchor.1);
                        }
                        gesture.set(update.next);
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
