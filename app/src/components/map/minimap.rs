use dioxus::prelude::*;

/// Longest edge of the minimap SVG, in screen pixels. The other edge is derived
/// from the content bounds' aspect ratio so the minimap never distorts the map.
const MINIMAP_MAX_EDGE: f64 = 160.0;

/// Half-length of the off-map chevron, in minimap pixels (apex to base).
const CHEVRON_HALF: f64 = 7.0;

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

/// Bottom-right viewport navigator. The outer box is the overall map (content)
/// bounds and fills the minimap edge-to-edge at a fixed scale — panning never
/// rescales it. The currently-visible world region is shown as the viewfinder:
/// a translucent rectangle while it overlaps the box (clipped to the box), or a
/// chevron pinned to the border pointing toward it once it is fully off-map.
/// Dragging inside the minimap recenters the main view on the corresponding
/// world point, which works in both states.
///
/// The host keeps its `Viewport` private and passes only plain values: the outer
/// box (`world_bounds`), the visible world rect (`visible`), and an `on_recenter`
/// callback that takes the world point to center the main view on.
#[component]
pub fn Minimap(
    /// Outer rectangle in world centimeters: `(min_x, min_y, max_x, max_y)`.
    /// This is the content bounds and maps 1:1 to the minimap box edges.
    world_bounds: (f64, f64, f64, f64),
    /// Currently-visible world rectangle: `(min_x, min_y, max_x, max_y)`.
    visible: (f64, f64, f64, f64),
    /// Called with a world point `(x, y)` to center the main view on.
    on_recenter: Callback<(f64, f64)>,
) -> Element {
    let (wmin_x, wmin_y, wmax_x, wmax_y) = world_bounds;
    let world_w = (wmax_x - wmin_x).max(1.0);
    let world_h = (wmax_y - wmin_y).max(1.0);

    // Fit the content bounds into the max edge, preserving aspect ratio. A single
    // scale (pixels per cm) keeps both axes consistent so the map isn't skewed,
    // and the content fills the box edge-to-edge with no inset margin.
    let scale = (MINIMAP_MAX_EDGE / world_w).min(MINIMAP_MAX_EDGE / world_h);
    let box_w = world_w * scale;
    let box_h = world_h * scale;

    // World -> minimap pixel mapping (relative to the world top-left).
    let to_px = move |x: f64, y: f64| ((x - wmin_x) * scale, (y - wmin_y) * scale);

    // Viewfinder rect in minimap px; may extend beyond the box on any side.
    let (vmin_x, vmin_y, vmax_x, vmax_y) = visible;
    let (vx0, vy0) = to_px(vmin_x, vmin_y);
    let (vx1, vy1) = to_px(vmax_x, vmax_y);

    // Does the viewfinder rect overlap the box at all? Touching counts as out.
    let overlaps = vx1 > 0.0 && vy1 > 0.0 && vx0 < box_w && vy0 < box_h;

    // Clipped viewfinder rect (only meaningful while it overlaps the box).
    let cx0 = vx0.max(0.0);
    let cy0 = vy0.max(0.0);
    let cx1 = vx1.min(box_w);
    let cy1 = vy1.min(box_h);
    let clip_w = (cx1 - cx0).max(0.0);
    let clip_h = (cy1 - cy0).max(0.0);

    // Off-map chevron: a triangle pinned where the ray from the box center to the
    // viewfinder center crosses the box border, pointing outward along that ray.
    let chevron = (!overlaps).then(|| {
        let bcx = box_w / 2.0;
        let bcy = box_h / 2.0;
        let vcx = (vx0 + vx1) / 2.0;
        let vcy = (vy0 + vy1) / 2.0;
        chevron_points(bcx, bcy, vcx, vcy, box_w, box_h)
    });

    // Translate a minimap-relative pixel position back to a world point and ask
    // the host to recenter on it. Centering the visible rect on the pointer is
    // the simplest mapping and feels natural for a grab-and-drag navigator, and
    // it works whether the viewfinder is on- or off-map (dragging toward the
    // content brings it back).
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
            width: "{box_w}",
            height: "{box_h}",
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
                width: "{box_w}",
                height: "{box_h}",
            }

            // Viewfinder: a clipped rect while overlapping, else an off-map
            // chevron pinned to the border pointing toward it.
            if let Some(points) = chevron {
                polygon { class: "map-minimap__chevron", points: "{points}" }
            } else if overlaps {
                rect {
                    class: "map-minimap__viewport",
                    x: "{cx0}",
                    y: "{cy0}",
                    width: "{clip_w}",
                    height: "{clip_h}",
                }
            }
        }
    }
}

/// Build an outward-pointing chevron (isoceles triangle) for an off-map
/// viewfinder, as an SVG `points` string in minimap pixels.
///
/// The apex sits where the ray from the box center `(bcx, bcy)` toward the
/// viewfinder center `(vcx, vcy)` crosses the box border; the triangle points
/// along that ray. The two base corners are offset from a point pulled slightly
/// back inside the border, perpendicular to the ray, so the chevron reads as an
/// arrow hugging the edge.
fn chevron_points(bcx: f64, bcy: f64, vcx: f64, vcy: f64, box_w: f64, box_h: f64) -> String {
    let dx = vcx - bcx;
    let dy = vcy - bcy;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let (ux, uy) = (dx / len, dy / len);

    // Distance from center to the border along the ray (axis-aligned box): the
    // smaller of the horizontal / vertical scalings reaches the nearer edge.
    let tx = if ux != 0.0 {
        (box_w / 2.0) / ux.abs()
    } else {
        f64::INFINITY
    };
    let ty = if uy != 0.0 {
        (box_h / 2.0) / uy.abs()
    } else {
        f64::INFINITY
    };
    let t = tx.min(ty);

    // Apex on the border; base sits CHEVRON_HALF back inside, fanned out by the
    // same amount perpendicular to the ray (perp of (ux,uy) is (-uy,ux)).
    let apex_x = bcx + ux * t;
    let apex_y = bcy + uy * t;
    let base_cx = apex_x - ux * CHEVRON_HALF;
    let base_cy = apex_y - uy * CHEVRON_HALF;
    let (px, py) = (-uy, ux);
    let b1x = base_cx + px * CHEVRON_HALF;
    let b1y = base_cy + py * CHEVRON_HALF;
    let b2x = base_cx - px * CHEVRON_HALF;
    let b2y = base_cy - py * CHEVRON_HALF;

    format!("{apex_x},{apex_y} {b1x},{b1y} {b2x},{b2y}")
}
