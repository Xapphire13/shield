use dioxus::prelude::*;
use shield_models::{DoorSwing, MapDoor};

/// Radius (in logical cm) of a door endpoint drag handle, shown only once the
/// door is selected. Sized consistently with `map_wall.rs`'s
/// `VERTEX_HANDLE_RADIUS_CM`.
const ENDPOINT_HANDLE_RADIUS_CM: f64 = 18.0;

/// Which endpoint of a [`MapDoor`] is being referenced/dragged. A door has
/// exactly two named endpoints (unlike a wall's arbitrary-length vertex
/// list), so this is a small enum rather than an index.
#[derive(Clone, Copy, PartialEq)]
pub enum Endpoint {
    Start,
    End,
}

/// Renders a single placed [`MapDoor`] as a standard architectural door
/// swing symbol: a straight line across the opening (`start`..`end`, the
/// same width the door occupies in a wall gap the user left) plus a
/// quarter-circle swing arc showing which side it opens toward. All
/// geometry is in logical world-space centimeters, same convention as
/// [`MapCameraMarker`](super::map_camera::MapCameraMarker) /
/// [`MapWallPath`](super::map_wall::MapWallPath).
///
/// Selectable via a pointer-down on the opening line; once selected (and in
/// edit mode) each endpoint gets an on-canvas drag handle for reshaping /
/// repositioning it. There is no whole-door drag — only individual
/// endpoints move, same "select-only on body, drag only via named handles"
/// pattern `MapWallPath` established for walls.
#[component]
pub fn MapDoorMarker(
    door: MapDoor,
    /// Whether this door is the current selection (shows endpoint handles +
    /// emphasis).
    #[props(default)]
    selected: bool,
    /// Whether the map is in edit mode (enables interaction). Outside edit
    /// mode the door is inert.
    #[props(default)]
    editing: bool,
    /// Fired on pointer-down on the opening line. The host uses this to
    /// select the door.
    #[props(default)]
    on_body_pointer_down: Option<Callback<Event<PointerData>>>,
    /// Fired on pointer-down on an endpoint handle, with which endpoint. The
    /// host uses this to start a per-endpoint drag.
    #[props(default)]
    on_endpoint_pointer_down: Option<Callback<(Endpoint, Event<PointerData>)>>,
) -> Element {
    let (open_x, open_y) = swing_open_point(&door);
    rsx! {
        g {
            class: "map-door",
            "data-selected": selected,
            "data-editing": editing,
            line {
                class: "map-door__opening",
                x1: "{door.start.x}",
                y1: "{door.start.y}",
                x2: "{door.end.x}",
                y2: "{door.end.y}",
                onpointerdown: move |evt: Event<PointerData>| {
                    if editing {
                        evt.stop_propagation();
                        if let Some(cb) = on_body_pointer_down {
                            cb.call(evt);
                        }
                    }
                },
            }
            line {
                class: "map-door__leaf",
                x1: "{door.start.x}",
                y1: "{door.start.y}",
                x2: "{open_x}",
                y2: "{open_y}",
            }
            path {
                class: "map-door__swing-arc",
                d: "{swing_arc_d(&door, open_x, open_y)}",
                fill: "none",
            }
            if selected && editing {
                circle {
                    class: "map-door__endpoint-handle",
                    cx: "{door.start.x}",
                    cy: "{door.start.y}",
                    r: "{ENDPOINT_HANDLE_RADIUS_CM}",
                    onpointerdown: move |evt: Event<PointerData>| {
                        evt.stop_propagation();
                        if let Some(cb) = on_endpoint_pointer_down {
                            cb.call((Endpoint::Start, evt));
                        }
                    },
                }
                circle {
                    class: "map-door__endpoint-handle",
                    cx: "{door.end.x}",
                    cy: "{door.end.y}",
                    r: "{ENDPOINT_HANDLE_RADIUS_CM}",
                    onpointerdown: move |evt: Event<PointerData>| {
                        evt.stop_propagation();
                        if let Some(cb) = on_endpoint_pointer_down {
                            cb.call((Endpoint::End, evt));
                        }
                    },
                }
            }
        }
    }
}

/// The open-leaf endpoint: `start` is the hinge, the leaf is `|end - start|`
/// long, swept 90° from the closed (`start`→`end`) direction toward the
/// swing side. Reuses the same North-clockwise-bearing screen-angle
/// convention `fov_cone_path` (in `map_camera.rs`) already establishes —
/// here applied to the door's own start->end direction rather than a stored
/// bearing field, since `MapDoor` has no bearing of its own.
fn swing_open_point(door: &MapDoor) -> (f64, f64) {
    let dx = (door.end.x - door.start.x) as f64;
    let dy = (door.end.y - door.start.y) as f64;
    let radius = (dx * dx + dy * dy).sqrt();
    let theta0 = dy.atan2(dx); // screen-space angle (clockwise from +x, y-down) of start->end
    let offset = match door.swing {
        DoorSwing::Left => -std::f64::consts::FRAC_PI_2,
        DoorSwing::Right => std::f64::consts::FRAC_PI_2,
    };
    let open_theta = theta0 + offset;
    (
        door.start.x as f64 + radius * open_theta.cos(),
        door.start.y as f64 + radius * open_theta.sin(),
    )
}

/// SVG arc path from `end` (closed-leaf tip) to the open-leaf tip, both on
/// the circle of radius `|end - start|` centered at the hinge (`start`) —
/// exactly a quarter circle (90°), so `large-arc-flag` is always 0. The sweep
/// flag matches the rotation direction used in `swing_open_point`: sweeping
/// from the closed angle `theta0` to `theta0 - 90°` (`Left`) is a decreasing-
/// angle traversal, which is the sweep-flag-0 ("counterclockwise on screen")
/// direction in SVG's y-down coordinate system; sweeping to `theta0 + 90°`
/// (`Right`) is increasing-angle, sweep-flag-1 ("clockwise on screen").
/// Hand-verified with `start = (0,0)`, `end = (100,0)`: `Left` opens to
/// `(0,-100)` (swings up/north, arc traced 0° -> -90°, sweep 0); `Right`
/// opens to `(0,100)` (swings down/south, arc traced 0° -> 90°, sweep 1).
fn swing_arc_d(door: &MapDoor, open_x: f64, open_y: f64) -> String {
    let radius = (((door.end.x - door.start.x) as f64).powi(2)
        + ((door.end.y - door.start.y) as f64).powi(2))
    .sqrt();
    let sweep = match door.swing {
        DoorSwing::Left => 0,
        DoorSwing::Right => 1,
    };
    format!(
        "M {} {} A {radius} {radius} 0 0 {sweep} {} {}",
        door.end.x, door.end.y, open_x, open_y
    )
}
