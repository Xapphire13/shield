use dioxus::prelude::*;
use shield_models::MapCamera;

/// Radius (in logical cm) of the camera marker dot. Drawn in logical/world
/// space so it scales with zoom along with everything else.
const MARKER_RADIUS_CM: f64 = 25.0;

/// Renders a single placed [`MapCamera`]: a marker dot at its `position` plus a
/// translucent field-of-view wedge.
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
pub fn MapCameraMarker(camera: MapCamera) -> Element {
    let cx = camera.position.x as f64;
    let cy = camera.position.y as f64;

    let cone_path = fov_cone_path(cx, cy, &camera.fov);

    rsx! {
        g { class: "map-camera",
            // Field-of-view wedge.
            path { class: "map-camera__fov", d: "{cone_path}" }

            // Camera marker.
            circle {
                class: "map-camera__marker",
                cx: "{cx}",
                cy: "{cy}",
                r: "{MARKER_RADIUS_CM}",
            }
        }
    }
}

/// Builds the SVG path for a field-of-view wedge centered at `(cx, cy)` in
/// logical cm. See the component docs for the bearing convention.
fn fov_cone_path(cx: f64, cy: f64, fov: &shield_models::FieldOfView) -> String {
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
