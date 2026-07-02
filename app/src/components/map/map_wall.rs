use dioxus::prelude::*;
use shield_models::MapWall;

/// Renders a single placed [`MapWall`] as an SVG path. All geometry is in
/// logical world-space centimeters — the parent applies the pan/zoom
/// transform, same convention as [`MapCameraMarker`](super::map_camera::MapCameraMarker).
///
/// No selection/vertex-editing support yet (that lands once walls become
/// selectable in a later PR) — this just draws the shape. The stroke color is
/// a fixed placeholder until the real `WallColor` palette lands.
#[component]
pub fn MapWallPath(wall: MapWall) -> Element {
    let d = wall_path_d(&wall);
    rsx! {
        path { class: "map-wall__stroke", d: "{d}", fill: "none" }
    }
}

/// Builds the SVG path string for a wall: `M x0 y0 L x1 y1 ...`, with a
/// trailing `Z` when closed (SVG auto-closes back to the `M` point, so the
/// first vertex is never re-listed).
fn wall_path_d(wall: &MapWall) -> String {
    let mut parts = Vec::with_capacity(wall.vertices.len());
    for (i, v) in wall.vertices.iter().enumerate() {
        let cmd = if i == 0 { "M" } else { "L" };
        parts.push(format!("{cmd} {} {}", v.x, v.y));
    }
    if wall.closed {
        parts.push("Z".to_string());
    }
    parts.join(" ")
}
