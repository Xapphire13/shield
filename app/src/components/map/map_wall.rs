use dioxus::prelude::*;
use shield_models::MapWall;

use crate::components::map::color_swatch_picker::WallColorCssExt;

stylance::import_crate_style!(style, "src/components/map/map_wall.module.css");

/// Radius (in logical cm) of a wall vertex drag handle, shown only once the
/// wall is selected. Sized to be easy to grab without visually overwhelming
/// the (thinner) wall stroke, similar in spirit to `MARKER_RADIUS_CM` in
/// [`map_camera`](super::map_camera).
const VERTEX_HANDLE_RADIUS_CM: f64 = 18.0;

/// Renders a single placed [`MapWall`] as an SVG path. All geometry is in
/// logical world-space centimeters — the parent applies the pan/zoom
/// transform, same convention as [`MapCameraMarker`](super::map_camera::MapCameraMarker).
///
/// Selectable via a pointer-down on an invisible, constant-width hit area
/// layered over the (purely decorative, world-scaled) visible stroke — see
/// `.hit_area` in the co-located CSS module. Once selected (and in edit mode) each
/// vertex gets an on-canvas drag handle for reshaping the path. There is no
/// whole-wall drag — only individual vertices move. The visible stroke's
/// color reflects the wall's chosen [`WallColor`](shield_models::WallColor),
/// fed in via a CSS custom property (see `.stroke` in the CSS module)
/// rather than an inline `stroke`, so the selection-highlight rule can still
/// override it through normal cascade/specificity.
#[component]
pub fn MapWallPath(
    wall: MapWall,
    /// Whether this wall is the current selection (shows vertex handles +
    /// emphasis).
    #[props(default)]
    selected: bool,
    /// Whether this wall currently responds to a pointer-down (select) and
    /// shows its vertex handles when selected. False while edit mode is on
    /// but a different tool is armed or a placement picker is open — without
    /// this, the wall would stay clickable underneath an unrelated tool.
    #[props(default)]
    interactive: bool,
    /// Fired on pointer-down on the wall's hit area. The host uses this to
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
            class: style::container,
            "data-selected": selected,
            "data-interactive": interactive,
            path {
                class: style::stroke,
                d: "{d}",
                fill: "none",
                style: "--wall-stroke-color: var(--wall-color-{wall.color.css_name()});",
            }
            // Invisible, constant-width click target layered over the visible
            // stroke — see `.hit_area` in the CSS module for why this is separate
            // from the (world-scaled, purely decorative) stroke above.
            path {
                class: style::hit_area,
                d: "{d}",
                fill: "none",
                onpointerdown: move |evt: Event<PointerData>| {
                    if interactive {
                        evt.stop_propagation();
                        if let Some(cb) = on_path_pointer_down {
                            cb.call(evt);
                        }
                    }
                },
            }
            if selected && interactive {
                for (i , v) in wall.vertices.iter().enumerate() {
                    circle {
                        key: "{i}",
                        class: style::vertex_handle,
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
