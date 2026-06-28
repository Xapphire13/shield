use dioxus::prelude::*;

/// Longest edge of the minimap SVG, in screen pixels. The other edge is derived
/// from the world bounds' aspect ratio so the minimap never distorts the map.
const MINIMAP_MAX_EDGE: f64 = 160.0;

/// DOM id of the minimap SVG element, used to measure its own bounding rect so
/// pointer positions can be made minimap-relative (see [`Minimap`]).
const MINIMAP_ID: &str = "map-minimap";

/// Read the minimap's bounding rect as `(left, top, width, height)` in viewport
/// pixels, or `None` if it isn't in the DOM yet.
fn minimap_rect() -> Option<(f64, f64, f64, f64)> {
    let element = web_sys::window()?
        .document()?
        .get_element_by_id(MINIMAP_ID)?;
    let rect = element.get_bounding_client_rect();
    Some((rect.left(), rect.top(), rect.width(), rect.height()))
}

/// Bottom-right viewport navigator. Draws the overall map bounds as an outer box
/// and the currently-visible world region as an inner filled rectangle. Dragging
/// inside the minimap recenters the main view on the corresponding world point.
///
/// The host keeps its `Viewport` private and passes only plain values: the outer
/// box (`world_bounds`), the inner indicator (`visible`), and an `on_recenter`
/// callback that takes the world point to center the main view on.
#[component]
pub fn Minimap(
    /// Outer rectangle in world centimeters: `(min_x, min_y, max_x, max_y)`.
    world_bounds: (f64, f64, f64, f64),
    /// Currently-visible world rectangle: `(min_x, min_y, max_x, max_y)`.
    visible: (f64, f64, f64, f64),
    /// Called with a world point `(x, y)` to center the main view on.
    on_recenter: Callback<(f64, f64)>,
) -> Element {
    let (wmin_x, wmin_y, wmax_x, wmax_y) = world_bounds;
    let world_w = (wmax_x - wmin_x).max(1.0);
    let world_h = (wmax_y - wmin_y).max(1.0);

    // Fit the world bounds into the max edge, preserving aspect ratio. A single
    // scale (pixels per cm) keeps both axes consistent so the map isn't skewed.
    let scale = (MINIMAP_MAX_EDGE / world_w).min(MINIMAP_MAX_EDGE / world_h);
    let svg_w = world_w * scale;
    let svg_h = world_h * scale;

    // World -> minimap pixel mapping (relative to the world top-left).
    let to_px = move |x: f64, y: f64| ((x - wmin_x) * scale, (y - wmin_y) * scale);

    let (vmin_x, vmin_y, vmax_x, vmax_y) = visible;
    let (ix, iy) = to_px(vmin_x, vmin_y);
    let (ix2, iy2) = to_px(vmax_x, vmax_y);
    let inner_w = (ix2 - ix).max(1.0);
    let inner_h = (iy2 - iy).max(1.0);

    // Translate a minimap-relative pixel position back to a world point and ask
    // the host to recenter on it. Centering the visible rect on the pointer is
    // the simplest mapping and feels natural for a grab-and-drag navigator.
    let recenter_from = move |px: f64, py: f64| {
        let wx = wmin_x + px / scale;
        let wy = wmin_y + py / scale;
        on_recenter.call((wx, wy));
    };

    // Whether a drag is in progress; while held, pointer moves keep recentering.
    let mut dragging = use_signal(|| false);

    // Convert a pointer event into a minimap-relative pixel position by measuring
    // the minimap's own rect. `client_coordinates` is viewport-relative and
    // target-independent — unlike `element_coordinates`, which is relative to
    // whichever SVG child is under the pointer and so jumps around mid-drag. The
    // minimap is fixed-position and fixed-size, so measuring on demand is fine.
    let pointer_px = move |evt: &PointerData| -> Option<(f64, f64)> {
        let (left, top, _, _) = minimap_rect()?;
        let client = evt.client_coordinates();
        Some((client.x - left, client.y - top))
    };

    rsx! {
        svg {
            id: MINIMAP_ID,
            class: "map-minimap",
            xmlns: "http://www.w3.org/2000/svg",
            // Explicit pixel dimensions: the minimap is fixed-size, and a
            // viewBox-less SVG without them collapses to its intrinsic size on
            // WebKit. width/height in user units match the coordinate space used
            // for the rects below.
            width: "{svg_w}",
            height: "{svg_h}",
            style: "touch-action: none;",
            onpointerdown: move |evt| {
                dragging.set(true);
                if let Some((px, py)) = pointer_px(&evt.data()) {
                    recenter_from(px, py);
                }
            },
            onpointermove: move |evt| {
                if *dragging.read()
                    && let Some((px, py)) = pointer_px(&evt.data())
                {
                    recenter_from(px, py);
                }
            },
            onpointerup: move |_| dragging.set(false),
            onpointercancel: move |_| dragging.set(false),

            // Outer box: the overall map bounds.
            rect {
                class: "map-minimap__bounds",
                x: "0",
                y: "0",
                width: "{svg_w}",
                height: "{svg_h}",
            }
            // Inner box: the currently-visible region.
            rect {
                class: "map-minimap__viewport",
                x: "{ix}",
                y: "{iy}",
                width: "{inner_w}",
                height: "{inner_h}",
            }
        }
    }
}
