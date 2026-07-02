use dioxus::prelude::*;
use shield_models::MapWall;

/// Radius (in logical cm) of a wall vertex drag handle, shown only once the
/// wall is selected. Sized to be easy to grab without visually overwhelming
/// the (thinner) wall stroke, similar in spirit to `MARKER_RADIUS_CM` in
/// [`map_camera`](super::map_camera).
const VERTEX_HANDLE_RADIUS_CM: f64 = 18.0;

/// Renders a single placed [`MapWall`] as an SVG path. All geometry is in
/// logical world-space centimeters — the parent applies the pan/zoom
/// transform, same convention as [`MapCameraMarker`](super::map_camera::MapCameraMarker).
///
/// Selectable via a pointer-down on the stroke; once selected (and in edit
/// mode) each vertex gets an on-canvas drag handle for reshaping the path.
/// There is no whole-wall drag — only individual vertices move. Recoloring
/// (the real `WallColor` palette) lands in a later PR; the stroke color here
/// is a fixed placeholder.
#[component]
pub fn MapWallPath(
    wall: MapWall,
    /// Whether this wall is the current selection (shows vertex handles +
    /// emphasis).
    #[props(default)]
    selected: bool,
    /// Whether the map is in edit mode (enables interaction). Outside edit
    /// mode the path is inert.
    #[props(default)]
    editing: bool,
    /// Fired on pointer-down on the wall's stroke. The host uses this to
    /// select the wall.
    #[props(default)]
    on_path_pointer_down: Option<Callback<Event<PointerData>>>,
    /// Fired on pointer-down on a vertex handle, with the index of the
    /// vertex. The host uses this to start a per-vertex drag.
    #[props(default)]
    on_vertex_pointer_down: Option<Callback<(usize, Event<PointerData>)>>,
) -> Element {
    let d = wall_path_d(&wall);
    rsx! {
        g {
            class: "map-wall",
            "data-selected": selected,
            "data-editing": editing,
            path {
                class: "map-wall__stroke",
                d: "{d}",
                fill: "none",
                onpointerdown: move |evt: Event<PointerData>| {
                    evt.stop_propagation();
                    if let Some(cb) = on_path_pointer_down {
                        cb.call(evt);
                    }
                },
            }
            if selected && editing {
                for (i , v) in wall.vertices.iter().enumerate() {
                    circle {
                        key: "{i}",
                        class: "map-wall__vertex-handle",
                        cx: "{v.x}",
                        cy: "{v.y}",
                        r: "{VERTEX_HANDLE_RADIUS_CM}",
                        onpointerdown: move |evt: Event<PointerData>| {
                            evt.stop_propagation();
                            if let Some(cb) = on_vertex_pointer_down {
                                cb.call((i, evt));
                            }
                        },
                    }
                }
            }
        }
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
