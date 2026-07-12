use dioxus::prelude::*;

stylance::import_crate_style!(style, "src/components/map/scale_bar.module.css");

/// Height of the tick marks (screen px) in the scale-bar SVG.
const TICK_HEIGHT: f64 = 8.0;

/// Persistent bottom-left scale bar: a line with a tick at each end and a
/// world-length label above it (e.g. "5 m"). The host sizes it to the current
/// grid spacing (see `Viewport::grid_spacing_cm`), so the bar always reads as
/// "this is one grid square" and steps through the same "nice" 1-2-5 lengths
/// as the grid when zooming.
#[component]
pub fn ScaleBar(
    /// On-screen length of the bar in pixels (world length in cm * zoom).
    width_px: f64,
    /// Formatted world length, e.g. "5 m".
    label: String,
) -> Element {
    rsx! {
        div { class: style::container,
            span { class: style::label, "{label}" }
            svg {
                class: style::ticks,
                width: "{width_px}",
                height: "{TICK_HEIGHT}",
                line {
                    class: style::stroke,
                    x1: "0.5",
                    y1: "0",
                    x2: "0.5",
                    y2: "{TICK_HEIGHT}",
                }
                line {
                    class: style::stroke,
                    x1: "0.5",
                    y1: "{TICK_HEIGHT / 2.0}",
                    x2: "{width_px - 0.5}",
                    y2: "{TICK_HEIGHT / 2.0}",
                }
                line {
                    class: style::stroke,
                    x1: "{width_px - 0.5}",
                    y1: "0",
                    x2: "{width_px - 0.5}",
                    y2: "{TICK_HEIGHT}",
                }
            }
        }
    }
}

/// Format a world length in centimeters (assumed to be a "nice" 1-2-5 value
/// from `nice_step_at_least`, so at most one decimal digit of meters) as a
/// scale-bar label, e.g. `50.0` -> `"0.5 m"`, `500.0` -> `"5 m"`.
pub fn format_scale_label(world_len_cm: f64) -> String {
    let meters = world_len_cm / 100.0;
    if meters.fract() == 0.0 {
        format!("{meters:.0} m")
    } else {
        format!("{meters:.1} m")
    }
}
