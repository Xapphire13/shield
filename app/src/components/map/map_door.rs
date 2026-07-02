use dioxus::prelude::*;
use shield_models::{DoorSwing, MapDoor};

/// Renders a single placed [`MapDoor`] as a standard architectural door
/// swing symbol: a straight line across the opening (`start`..`end`, the
/// same width the door occupies in a wall gap the user left) plus a
/// quarter-circle swing arc showing which side it opens toward. All
/// geometry is in logical world-space centimeters, same convention as
/// [`MapCameraMarker`](super::map_camera::MapCameraMarker) /
/// [`MapWallPath`](super::map_wall::MapWallPath).
///
/// No selection/editing support yet (lands in a later PR) — this just draws
/// the shape, always with a fixed default swing until then.
#[component]
pub fn MapDoorMarker(door: MapDoor) -> Element {
    let (open_x, open_y) = swing_open_point(&door);
    rsx! {
        g { class: "map-door",
            line {
                class: "map-door__opening",
                x1: "{door.start.x}",
                y1: "{door.start.y}",
                x2: "{door.end.x}",
                y2: "{door.end.y}",
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
