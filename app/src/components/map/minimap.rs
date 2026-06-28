use dioxus::prelude::*;

/// Longest edge of the minimap SVG, in screen pixels. The other edge is derived
/// from the content bounds' aspect ratio so the minimap never distorts the map.
const MINIMAP_MAX_EDGE: f64 = 160.0;

/// Arm length of the off-map chevron glyph, in minimap pixels — each of the two
/// arms runs this far back from the tip of the `‹`/`›`/`^`/`v` shape.
const CHEVRON_ARM: f64 = 5.0;

/// How far the chevron tip is inset from the box edge it hugs, in minimap pixels.
const CHEVRON_INSET: f64 = 6.0;

/// Radius of a placed-camera dot on the minimap, in minimap pixels.
const CAMERA_DOT_RADIUS: f64 = 2.0;

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
/// Panning the main view is done by grabbing and dragging the viewfinder
/// rectangle; a pointer-down that misses it does nothing. When the viewfinder is
/// fully off-map (chevron showing) there is no recenter interaction — the
/// off-map recenter UX is intentionally deferred.
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
    /// Placed camera positions in world centimeters, drawn as dots. Always
    /// inside `world_bounds`, so they need no clipping.
    cameras: Vec<(f64, f64)>,
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

    // Camera dots: each position through the same content->minimap scale. They
    // always sit inside the content bounds, so no clipping is needed.
    let dots: Vec<(f64, f64)> = cameras.iter().map(|&(x, y)| to_px(x, y)).collect();

    // Off-map chevron: snapped to one of 8 fixed directions from the viewfinder
    // center's position relative to the box, pinned to the matching edge/corner.
    let chevron =
        (!overlaps).then(|| chevron_path((vx0 + vx1) / 2.0, (vy0 + vy1) / 2.0, box_w, box_h));

    // Translate a minimap-relative pixel point (the desired viewfinder center)
    // back to a world point and ask the host to recenter the main view on it.
    let recenter_from = move |px: f64, py: f64| {
        let wx = wmin_x + px / scale;
        let wy = wmin_y + py / scale;
        on_recenter.call((wx, wy));
    };

    // Viewfinder center in minimap px (only meaningful while it overlaps the box,
    // i.e. while it is grabbable). Used for hit-testing the grab and to hold the
    // pointer's offset from the center so the viewfinder tracks the pointer
    // without jumping on grab.
    let vcx = (vx0 + vx1) / 2.0;
    let vcy = (vy0 + vy1) / 2.0;

    // While a drag is in progress, the grab offset (pointer minus viewfinder
    // center, in minimap px) captured on pointer-down; `None` when not dragging.
    // Holding the offset keeps the grabbed point under the pointer for the whole
    // drag instead of snapping the center to the pointer.
    let mut grab_offset = use_signal(|| None::<(f64, f64)>);

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

    // Hit-test a minimap-relative pixel point against the (unclipped) viewfinder
    // rect. Only the on-box viewfinder is grabbable; when it is fully off-map the
    // chevron is showing and there is no drag interaction (deferred).
    let in_viewfinder = move |px: f64, py: f64| px >= vx0 && px <= vx1 && py >= vy0 && py <= vy1;

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
                // Only a grab that lands on the viewfinder rect starts a drag; a
                // press elsewhere in the minimap does nothing. (When the
                // viewfinder is off-map the chevron shows and there is no grab —
                // the off-map recenter UX is intentionally deferred.)
                if overlaps
                    && let Some((px, py)) = pointer_px(&evt.data())
                    && in_viewfinder(px, py)
                {
                    grab_offset.set(Some((px - vcx, py - vcy)));
                }
            },
            onpointermove: move |evt| {
                // Drag: keep the grabbed point under the pointer by moving the
                // viewfinder center to `pointer - offset`, then recenter on it.
                if let Some((ox, oy)) = *grab_offset.read()
                    && let Some((px, py)) = pointer_px(&evt.data())
                {
                    recenter_from(px - ox, py - oy);
                }
            },
            onpointerup: move |_| grab_offset.set(None),
            onpointercancel: move |_| grab_offset.set(None),

            // Outer box: the overall map bounds.
            rect {
                class: "map-minimap__bounds",
                x: "0",
                y: "0",
                width: "{box_w}",
                height: "{box_h}",
            }

            // Placed cameras as small dots.
            for (dx, dy) in dots.iter().copied() {
                circle {
                    class: "map-minimap__camera",
                    cx: "{dx}",
                    cy: "{dy}",
                    r: "{CAMERA_DOT_RADIUS}",
                }
            }

            // Viewfinder: a clipped rect while overlapping, else an off-map
            // chevron pinned to the border pointing toward it.
            if let Some(path) = chevron {
                path { class: "map-minimap__chevron", d: "{path}" }
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

/// Build an open chevron glyph (`›`/`‹`/`^`/`v` or a diagonal corner variant)
/// for an off-map viewfinder, as an SVG path `d` string in minimap pixels.
///
/// Direction snaps to one of 8 fixed orientations from the viewfinder center's
/// position relative to the box `[0,w] x [0,h]`: each axis contributes a sign
/// (`vcx < 0` Left / `> w` Right; `vcy < 0` Up / `> h` Down), giving a cardinal
/// (one axis off) or diagonal (both off) direction — never a continuous angle.
/// The chevron is pinned to the matching edge or corner (inset), sliding along a
/// cardinal edge to track the viewfinder. It is rendered open (two arms meeting
/// at the tip), so the path is `tip - arm` -> `tip` -> `tip - arm` rotated by
/// the perpendicular: arms run back from the tip at ±45° around the point
/// direction `(ux, uy)` (a unit or diagonal-unit vector).
fn chevron_path(vcx: f64, vcy: f64, box_w: f64, box_h: f64) -> String {
    // Per-axis sign of the off-box direction (-1 / 0 / +1).
    let sx = if vcx < 0.0 {
        -1.0
    } else if vcx > box_w {
        1.0
    } else {
        0.0
    };
    let sy = if vcy < 0.0 {
        -1.0
    } else if vcy > box_h {
        1.0
    } else {
        0.0
    };

    // Tip position: pin to the off edge(s); slide along the on-axis to track the
    // viewfinder, clamped to stay inside the box with the inset.
    let lo = CHEVRON_INSET;
    let tip_x = match sx {
        s if s < 0.0 => CHEVRON_INSET,
        s if s > 0.0 => box_w - CHEVRON_INSET,
        _ => vcx.clamp(lo, box_w - lo),
    };
    let tip_y = match sy {
        s if s < 0.0 => CHEVRON_INSET,
        s if s > 0.0 => box_h - CHEVRON_INSET,
        _ => vcy.clamp(lo, box_h - lo),
    };

    // Point direction (unit for cardinals, diagonal for corners). At least one
    // axis is non-zero because the chevron only renders when fully off-box.
    let (ux, uy) = if sx != 0.0 && sy != 0.0 {
        let inv = 1.0 / std::f64::consts::SQRT_2;
        (sx * inv, sy * inv)
    } else {
        (sx, sy)
    };

    // The two arms run back from the tip at ±45° to the point direction:
    // rotate `-(ux, uy) * arm` by ±45° (rotation by ±45° of a vector (x, y) is
    // (x∓y, ±x+y) / √2).
    let bx = -ux * CHEVRON_ARM;
    let by = -uy * CHEVRON_ARM;
    let r = 1.0 / std::f64::consts::SQRT_2;
    let a1x = tip_x + (bx - by) * r;
    let a1y = tip_y + (bx + by) * r;
    let a2x = tip_x + (bx + by) * r;
    let a2y = tip_y + (-bx + by) * r;

    format!("M {a1x} {a1y} L {tip_x} {tip_y} L {a2x} {a2y}")
}
