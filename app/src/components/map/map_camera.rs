use dioxus::prelude::*;
use shield_models::{FieldOfView, MapCamera};

/// Radius (in logical cm) of the camera marker dot. Drawn in logical/world
/// space so it scales with zoom along with everything else.
const MARKER_RADIUS_CM: f64 = 25.0;

/// Renders a single placed [`MapCamera`]: a marker dot at its `position` plus a
/// translucent field-of-view wedge. In edit mode the marker is interactive
/// (selectable / draggable) and, when selected, exposes on-canvas handles for
/// aiming and ranging the FOV cone.
///
/// All geometry is emitted in **logical world space (centimeters)**; the parent
/// [`MapView`](super::map_view::MapView) wraps these in a transform that maps cm
/// to screen pixels, so this component never deals with pan/zoom directly.
///
/// ## FOV cone math
/// Bearings are true-North relative and clockwise (0 = up/North). SVG's y axis
/// points **down**, so a screen-space angle `theta` measured clockwise from the
/// +x axis maps to `(cos theta, sin theta)`. Converting a North-clockwise
/// bearing `b` to that convention: North (up) is `-y`, i.e. screen angle -90;
/// rotating clockwise increases both. Hence `theta = b - 90` (degrees). The two
/// cone edges sit at `direction_deg ± angle_deg / 2`, each `range` cm long, and
/// are closed with a circular arc of radius `range`.
#[component]
pub fn MapCameraMarker(
    camera: MapCamera,
    /// Whether this camera is the current selection (shows handles + emphasis).
    #[props(default)]
    selected: bool,
    /// Whether the map is in edit mode (enables interaction). Outside edit mode
    /// the marker is inert.
    #[props(default)]
    editing: bool,
    /// Whether the referenced [`Camera`](shield_models::Camera) no longer exists
    /// (placed reference is dangling). Rendered in a distinct "unknown" style.
    #[props(default)]
    orphaned: bool,
    /// Fired on pointer-down on the marker body. The host uses this to start a
    /// move drag and to disambiguate from a canvas pan.
    #[props(default)]
    on_body_pointer_down: Option<Callback<Event<PointerData>>>,
    /// Fired on pointer-down on the aim handle (sets FOV direction).
    #[props(default)]
    on_aim_pointer_down: Option<Callback<Event<PointerData>>>,
    /// Fired on pointer-down on the range handle (sets FOV range).
    #[props(default)]
    on_range_pointer_down: Option<Callback<Event<PointerData>>>,
) -> Element {
    let cx = camera.position.x as f64;
    let cy = camera.position.y as f64;

    let cone_path = fov_cone_path(cx, cy, &camera.fov);

    // World-space endpoints of the two interactive handles, placed along the
    // cone's center line: the aim handle at half range, the range handle at the
    // cone tip.
    let center_deg = camera.fov.direction_deg as f64 - 90.0;
    let center = center_deg.to_radians();
    let range = camera.fov.range as f64;
    let aim_x = cx + (range * 0.5) * center.cos();
    let aim_y = cy + (range * 0.5) * center.sin();
    let range_x = cx + range * center.cos();
    let range_y = cy + range * center.sin();

    let mut group_class = String::from("map-camera");
    if selected {
        group_class.push_str(" map-camera--selected");
    }
    if orphaned {
        group_class.push_str(" map-camera--orphaned");
    }
    if editing {
        group_class.push_str(" map-camera--editing");
    }

    rsx! {
        g { class: "{group_class}",
            // Field-of-view wedge.
            path { class: "map-camera__fov", d: "{cone_path}" }

            // Selected cameras get on-canvas handles for direct manipulation.
            if selected && editing {
                // Guide line from the marker to the aim/range handles.
                line {
                    class: "map-camera__guide",
                    x1: "{cx}",
                    y1: "{cy}",
                    x2: "{range_x}",
                    y2: "{range_y}",
                }
                // Aim handle (rotates the cone).
                circle {
                    class: "map-camera__handle map-camera__handle--aim",
                    cx: "{aim_x}",
                    cy: "{aim_y}",
                    r: "{MARKER_RADIUS_CM * 0.8}",
                    onpointerdown: move |evt: Event<PointerData>| {
                        evt.stop_propagation();
                        if let Some(cb) = on_aim_pointer_down {
                            cb.call(evt);
                        }
                    },
                }
                // Range handle (lengthens / shortens the cone).
                circle {
                    class: "map-camera__handle map-camera__handle--range",
                    cx: "{range_x}",
                    cy: "{range_y}",
                    r: "{MARKER_RADIUS_CM * 0.8}",
                    onpointerdown: move |evt: Event<PointerData>| {
                        evt.stop_propagation();
                        if let Some(cb) = on_range_pointer_down {
                            cb.call(evt);
                        }
                    },
                }
            }

            // Camera marker. In edit mode a pointer-down here starts a move and
            // is stopped from bubbling so the canvas does not also start a pan.
            circle {
                class: "map-camera__marker",
                cx: "{cx}",
                cy: "{cy}",
                r: "{MARKER_RADIUS_CM}",
                onpointerdown: move |evt: Event<PointerData>| {
                    if editing {
                        evt.stop_propagation();
                        if let Some(cb) = on_body_pointer_down {
                            cb.call(evt);
                        }
                    }
                },
            }
        }
    }
}

/// Builds the SVG path for a field-of-view wedge centered at `(cx, cy)` in
/// logical cm. See the component docs for the bearing convention.
fn fov_cone_path(cx: f64, cy: f64, fov: &FieldOfView) -> String {
    let range = fov.range as f64;
    let half_angle = fov.angle_deg as f64 / 2.0;

    // North-clockwise bearing -> screen angle (clockwise from +x, y-down).
    let start_deg = fov.direction_deg as f64 - half_angle - 90.0;
    let end_deg = fov.direction_deg as f64 + half_angle - 90.0;

    let start = start_deg.to_radians();
    let end = end_deg.to_radians();

    let x1 = cx + range * start.cos();
    let y1 = cy + range * start.sin();
    let x2 = cx + range * end.cos();
    let y2 = cy + range * end.sin();

    // large-arc flag: set when the cone spans more than 180 degrees.
    let large_arc = if fov.angle_deg > 180 { 1 } else { 0 };
    // sweep flag 1 = clockwise (positive angle direction in y-down space).
    let sweep = 1;

    format!("M {cx} {cy} L {x1} {y1} A {range} {range} 0 {large_arc} {sweep} {x2} {y2} Z")
}
