//! The cursor-following world-coordinate readout shown while placing/drawing
//! or dragging a vertex.

use dioxus::prelude::*;

use crate::components::map::interaction::{DragPreview, Tool};
use crate::components::map::viewport::Viewport;

stylance::import_crate_style!(style, "src/components/map/coord_readout.module.css");

/// Coordinate readout for placement tools and vertex drags: follows the
/// pointer, offset slightly so the label doesn't sit directly under the
/// cursor/finger. Shown while a placement tool is armed, and while dragging an
/// existing camera, wall vertex, or door endpoint (using the same previewed
/// position the canvas is rendering, not a fresh screen-to-world lookup, so
/// the readout always matches what's on screen). Positioned in canvas-relative
/// pixels, so it must be rendered inside the canvas frame (not the svg).
#[component]
pub fn CoordReadout(
    tool: Signal<Tool>,
    cursor_pos: Signal<Option<(f64, f64)>>,
    drag_preview: Signal<DragPreview>,
    viewport: Signal<Viewport>,
) -> Element {
    rsx! {
        if let Some((cx, cy)) = *cursor_pos.read() {
            {
                let coords = drag_preview.read().dragged_vertex_position()
                    .map(|position| (position.x, position.y))
                    .or_else(|| {
                        if matches!(*tool.read(), Tool::Select) {
                            None
                        } else {
                            let (wx, wy) = viewport.read().screen_to_world(cx, cy);
                            Some((wx.round() as i32, wy.round() as i32))
                        }
                    });
                rsx! {
                    if let Some((wx, wy)) = coords {
                        {
                            let mx = wx as f64 / 100.0;
                            let my = wy as f64 / 100.0;
                            rsx! {
                                div {
                                    class: style::coord_readout,
                                    style: "left: {cx + 14.0}px; top: {cy + 14.0}px;",
                                    "{mx:.2}, {my:.2} m",
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
