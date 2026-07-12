//! The in-progress wall draft and door placement rubber-band: transient
//! cursor-follow feedback while drawing/placing. A read-only view over the
//! host's signals — committing (or cancelling) stays with `MapView`'s
//! handlers.

use dioxus::prelude::*;

use crate::components::map::canvas_gestures::DRAFT_VERTEX_HIT_RADIUS_PX;
use crate::components::map::geometry::distance;
use crate::components::map::interaction::Tool;
use crate::components::map::map_camera::MARKER_RADIUS_CM;
use crate::components::map::viewport::Viewport;

stylance::import_crate_style!(style, "src/components/map/draft_overlay.module.css");

/// In-progress placement/drawing preview, rendered inside the world-space
/// content group so it pans and zooms with everything else.
#[component]
pub fn DraftOverlay(
    tool: Signal<Tool>,
    cursor_pos: Signal<Option<(f64, f64)>>,
    viewport: Signal<Viewport>,
) -> Element {
    rsx! {
        // --- In-progress door placement preview ---
        // A rubber-band line from the already-placed start point to
        // the live cursor position while the second click is still
        // pending, same technique the wall draft's rubber-band
        // uses. Reuses `.rubber_band` directly
        // (same visual language: "a tentative, not-yet-committed
        // line") rather than a near-duplicate class.
        if let Tool::PlaceDoor { start: Some(point) } = &*tool.read()
            && let Some((cx, cy)) = *cursor_pos.read()
        {
            {
                let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                rsx! {
                    line {
                        class: style::rubber_band,
                        x1: "{point.x}",
                        y1: "{point.y}",
                        x2: "{wx}",
                        y2: "{wy}",
                    }
                }
            }
        }

        // --- In-progress wall draft ---
        // Purely the live-drawing preview for the active `Tool::DrawWall`
        // draft; nothing is committed until the path finishes.
        if let Tool::DrawWall { vertices } = &*tool.read() {
            // Committed segments so far: a plain open polyline,
            // never closed while still drafting.
            if vertices.len() >= 2 {
                {
                    let mut parts = Vec::with_capacity(vertices.len());
                    for (i, v) in vertices.iter().enumerate() {
                        let cmd = if i == 0 { "M" } else { "L" };
                        parts.push(format!("{cmd} {} {}", v.x, v.y));
                    }
                    let d = parts.join(" ");
                    rsx! {
                        path { class: style::path, d: "{d}" }
                    }
                }
            }

            // Rubber-band segment from the last committed vertex to
            // the live cursor position, derived from `cursor_pos`
            // (reused rather than tracked separately).
            if let (Some(last), Some((cx, cy))) = (vertices.last(), *cursor_pos.read()) {
                {
                    let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                    rsx! {
                        line {
                            class: style::rubber_band,
                            x1: "{last.x}",
                            y1: "{last.y}",
                            x2: "{wx}",
                            y2: "{wy}",
                        }
                    }
                }
            }

            // A small dot at each committed vertex.
            for (i , v) in vertices.iter().enumerate() {
                circle {
                    key: "{i}",
                    class: style::vertex,
                    cx: "{v.x}",
                    cy: "{v.y}",
                    r: "{MARKER_RADIUS_CM * 0.3}",
                }
            }

            // Once there are enough vertices to close a loop,
            // highlight the first vertex as the close-loop target,
            // with a hover affordance once the cursor is actually
            // within the auto-close hit radius (same threshold the
            // pointerdown handler uses to commit the close).
            if vertices.len() >= 3
                && let Some(first) = vertices.first()
            {
                {
                    let in_range = cursor_pos.read().is_some_and(|(cx, cy)| {
                        let (v0_sx, v0_sy) = viewport
                            .read()
                            .world_to_screen(first.x as f64, first.y as f64);
                        distance(cx, cy, v0_sx, v0_sy) <= DRAFT_VERTEX_HIT_RADIUS_PX
                    });
                    rsx! {
                        circle {
                            class: style::close_target,
                            "data-in-range": in_range,
                            cx: "{first.x}",
                            cy: "{first.y}",
                            r: "{MARKER_RADIUS_CM}",
                        }
                    }
                }
            }

            // Once there are enough vertices to form a wall, highlight the
            // last vertex as the finish target — click it to end the path as
            // an open wall (same threshold the pointerdown handler uses to
            // commit the finish). Same affordance pattern as the close-loop
            // target above.
            if vertices.len() >= 2
                && let Some(last) = vertices.last()
            {
                {
                    let in_range = cursor_pos.read().is_some_and(|(cx, cy)| {
                        let (v_sx, v_sy) = viewport
                            .read()
                            .world_to_screen(last.x as f64, last.y as f64);
                        distance(cx, cy, v_sx, v_sy) <= DRAFT_VERTEX_HIT_RADIUS_PX
                    });
                    rsx! {
                        circle {
                            class: style::finish_target,
                            "data-in-range": in_range,
                            cx: "{last.x}",
                            cy: "{last.y}",
                            r: "{MARKER_RADIUS_CM}",
                        }
                    }
                }
            }
        }
    }
}
