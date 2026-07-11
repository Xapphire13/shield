//! Pure world-space geometry helpers shared by the map canvas: distances,
//! bearings, drag deltas, and content bounding boxes. No Dioxus or DOM
//! dependency.

use shield_models::{MapCamera, MapDoor, MapWall, Point};

use crate::components::map::map_camera::MARKER_RADIUS_CM;

/// Minimum half-extent (cm) folded around each camera so a single tiny cone
/// still yields a non-degenerate (non-zero-size) box.
const MIN_CONTENT_HALF_EXTENT: f64 = 50.0;

/// Step (degrees) used when sampling a FOV arc for the bounding box. Small
/// enough to tightly hug the arc, cheap enough for a handful of cameras.
const ARC_SAMPLE_STEP_DEG: f64 = 2.0;

/// Euclidean distance between two points.
pub fn distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

/// World-space bearing from a camera center to a world point, expressed as a
/// true-North clockwise direction in whole degrees (0 = up/North), matching the
/// FOV convention. Inverse of the cone math: screen angle `theta` (clockwise
/// from +x, y-down) relates to bearing `b` by `b = theta + 90`.
pub fn bearing_to(cx: f64, cy: f64, wx: f64, wy: f64) -> u16 {
    let theta = (wy - cy).atan2(wx - cx).to_degrees();
    let bearing = (theta + 90.0).rem_euclid(360.0);
    bearing.round() as u16
}

/// Applies a zoom-scaled screen-pixel drag delta (`(last_x, last_y)` -> `(cx,
/// cy)`) to a world-space `base` point, rounding to the nearest whole
/// centimeter. Shared by every per-element drag gesture (camera move, wall
/// vertex, door endpoint) since the underlying screen-to-world delta math is
/// identical regardless of what's being dragged.
pub fn apply_drag_delta(
    base: Point,
    cx: f64,
    cy: f64,
    last_x: f64,
    last_y: f64,
    zoom: f64,
) -> Point {
    let dx = (cx - last_x) / zoom;
    let dy = (cy - last_y) / zoom;
    Point {
        x: base.x + dx.round() as i32,
        y: base.y + dy.round() as i32,
    }
}

/// World-space bounding box `(min_x, min_y, max_x, max_y)` in centimeters that
/// tightly encloses what is actually drawn for every camera (the marker disc
/// (`position` ± [`MARKER_RADIUS_CM`]) and the FOV wedge (apex at `position`
/// plus its arc)), every wall vertex, and every door endpoint. Returns `None`
/// only when `cameras`, `walls`, and `doors` are all empty (callers keep the
/// default view).
///
/// The wedge is directional, so a symmetric ±`range` square would pad the sides
/// the cone doesn't face and mis-center the fit. Instead the arc is *sampled*
/// (matching the cone math in [`map_camera`](super::map_camera)): screen angle
/// (clockwise from +x, y-down) is `bearing - 90`, the wedge spans
/// `direction_deg ± angle_deg / 2`, and each sample is `position + range *
/// (cos θ, sin θ)`. Sampling avoids fiddly cardinal-crossing extrema math and
/// stays cheap and reusable — e.g. a minimap can call it to size its world rect.
///
/// A wall vertex or door endpoint is a literal point (unlike a camera
/// marker's disc), so no extra padding is folded in for either — the
/// stroke width is visually negligible for bounds purposes.
pub fn content_bounds(
    cameras: &[MapCamera],
    walls: &[MapWall],
    doors: &[MapDoor],
) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    // Whether `fold` has run at least once. A non-empty `walls`/`doors` slice
    // doesn't guarantee this — a wall with no vertices contributes nothing —
    // so this (not the slices' emptiness) is what actually determines whether
    // there's real content to report bounds for.
    let mut has_content = false;

    let mut fold = |x: f64, y: f64| {
        has_content = true;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };

    for camera in cameras {
        let cx = camera.position.x as f64;
        let cy = camera.position.y as f64;

        // Marker disc, with a small minimum so a degenerate cone still has area.
        let pad = MARKER_RADIUS_CM.max(MIN_CONTENT_HALF_EXTENT);
        fold(cx - pad, cy - pad);
        fold(cx + pad, cy + pad);

        // FOV wedge: apex plus sampled arc (range away along each sampled angle).
        let range = camera.fov.range as f64;
        let half = camera.fov.angle_deg as f64 / 2.0;
        let start = camera.fov.direction_deg as f64 - half - 90.0;
        let end = camera.fov.direction_deg as f64 + half - 90.0;

        let mut deg = start;
        loop {
            let theta = deg.min(end).to_radians();
            fold(cx + range * theta.cos(), cy + range * theta.sin());
            if deg >= end {
                break;
            }
            deg += ARC_SAMPLE_STEP_DEG;
        }
    }

    for wall in walls {
        for vertex in &wall.vertices {
            fold(vertex.x as f64, vertex.y as f64);
        }
    }

    for door in doors {
        fold(door.start.x as f64, door.start.y as f64);
        fold(door.end.x as f64, door.end.y as f64);
    }

    has_content.then_some((min_x, min_y, max_x, max_y))
}

/// Whether `outer` fully contains `inner` (both `(min_x, min_y, max_x, max_y)`).
pub fn fully_contains_bounds(outer: (f64, f64, f64, f64), inner: (f64, f64, f64, f64)) -> bool {
    outer.0 <= inner.0 && outer.1 <= inner.1 && outer.2 >= inner.2 && outer.3 >= inner.3
}

#[cfg(test)]
mod tests {
    use shield_models::{DoorSwing, FieldOfView, WallColor};

    use super::*;

    #[test]
    fn distance_matches_pythagoras() {
        assert_eq!(distance(0.0, 0.0, 3.0, 4.0), 5.0);
        assert_eq!(distance(1.0, 1.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn bearing_to_maps_cardinal_directions() {
        // North (up, -y) / East (+x) / South (+y) / West (-x).
        assert_eq!(bearing_to(0.0, 0.0, 0.0, -10.0), 0);
        assert_eq!(bearing_to(0.0, 0.0, 10.0, 0.0), 90);
        assert_eq!(bearing_to(0.0, 0.0, 0.0, 10.0), 180);
        assert_eq!(bearing_to(0.0, 0.0, -10.0, 0.0), 270);
    }

    #[test]
    fn apply_drag_delta_scales_screen_pixels_by_zoom() {
        let base = Point { x: 100, y: 100 };
        // A (10, 20) screen-px drag at zoom 2.0 is a (5, 10) cm world move.
        let moved = apply_drag_delta(base, 10.0, 20.0, 0.0, 0.0, 2.0);
        assert_eq!((moved.x, moved.y), (105, 110));
    }

    #[test]
    fn apply_drag_delta_rounds_to_whole_centimeters() {
        // 1 screen px at zoom 3.0 is a third of a cm — rounds away.
        let moved = apply_drag_delta(Point { x: 0, y: 0 }, 1.0, 1.0, 0.0, 0.0, 3.0);
        assert_eq!((moved.x, moved.y), (0, 0));
        // 2 screen px is two thirds — rounds up.
        let moved = apply_drag_delta(Point { x: 0, y: 0 }, 2.0, 2.0, 0.0, 0.0, 3.0);
        assert_eq!((moved.x, moved.y), (1, 1));
    }

    #[test]
    fn content_bounds_is_none_for_no_content() {
        assert_eq!(content_bounds(&[], &[], &[]), None);
        // A wall with no vertices contributes nothing, so it alone is still
        // "no content".
        let empty_wall = MapWall {
            id: "w".into(),
            vertices: vec![],
            closed: false,
            color: WallColor::default(),
        };
        assert_eq!(content_bounds(&[], &[empty_wall], &[]), None);
    }

    #[test]
    fn content_bounds_covers_camera_marker_and_fov_reach() {
        let camera = MapCamera {
            camera_id: "c".into(),
            position: Point { x: 0, y: 0 },
            fov: FieldOfView {
                direction_deg: 0, // Facing North (-y).
                angle_deg: 70,
                range: 500,
            },
        };
        let (min_x, min_y, max_x, max_y) = content_bounds(&[camera], &[], &[]).unwrap();
        // The south side the cone doesn't face is bounded by the marker pad
        // alone (min half-extent 50).
        assert_eq!(max_y, 50.0);
        // The wedge reaches wider than the pad east/west...
        assert!(min_x < -50.0 && max_x > 50.0, "x = {min_x}..{max_x}");
        // ...and the sampled arc reaches (almost exactly) the full range due
        // North.
        assert!((-500.0..=-499.0).contains(&min_y), "min_y = {min_y}");
    }

    #[test]
    fn content_bounds_covers_wall_vertices_and_door_endpoints() {
        let wall = MapWall {
            id: "w".into(),
            vertices: vec![Point { x: -20, y: 5 }, Point { x: 30, y: 40 }],
            closed: false,
            color: WallColor::default(),
        };
        let door = MapDoor {
            id: "d".into(),
            start: Point { x: 0, y: -100 },
            end: Point { x: 90, y: 10 },
            swing: DoorSwing::default(),
        };
        let bounds = content_bounds(&[], &[wall], &[door]).unwrap();
        assert_eq!(bounds, (-20.0, -100.0, 90.0, 40.0));
    }

    #[test]
    fn fully_contains_bounds_requires_all_edges() {
        let outer = (0.0, 0.0, 100.0, 100.0);
        assert!(fully_contains_bounds(outer, (10.0, 10.0, 90.0, 90.0)));
        assert!(fully_contains_bounds(outer, outer));
        assert!(!fully_contains_bounds(outer, (-1.0, 10.0, 90.0, 90.0)));
        assert!(!fully_contains_bounds(outer, (10.0, 10.0, 101.0, 90.0)));
    }
}
