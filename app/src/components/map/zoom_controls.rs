use dioxus::prelude::*;

/// Persistent bottom-right zoom control: `[ − ] [ NN% ] [ + ]`. Always rendered
/// (even when the minimap auto-hides), since zooming in with `+` can bring the
/// hidden minimap back. It is fixed at the base bottom-right offset and the
/// minimap stacks above it (see `.map-minimap` / `.map-zoom-controls` in CSS).
///
/// The host keeps its `Viewport` private: this component only reports button
/// intent (`on_zoom_in` / `on_zoom_out`, which zoom around the canvas center)
/// and renders the supplied zoom percentage.
#[component]
pub fn ZoomControls(
    /// Current zoom as a percentage to display, already rounded by the host
    /// (e.g. `25` renders as `25%`).
    percent: i64,
    /// Called when the minus button is pressed (zoom out).
    on_zoom_out: Callback<()>,
    /// Called when the plus button is pressed (zoom in).
    on_zoom_in: Callback<()>,
    /// Called when the percentage label is clicked (reset to 100% / auto-fit).
    on_reset_zoom: Callback<()>,
) -> Element {
    rsx! {
        div { class: "map-zoom-controls",
            button {
                class: "map-zoom-controls__button",
                "aria-label": "Zoom out",
                onclick: move |_| on_zoom_out.call(()),
                "−"
            }
            button {
                class: "map-zoom-controls__percent",
                "aria-label": "Reset zoom to fit",
                onclick: move |_| on_reset_zoom.call(()),
                "{percent}%"
            }
            button {
                class: "map-zoom-controls__button",
                "aria-label": "Zoom in",
                onclick: move |_| on_zoom_in.call(()),
                "+"
            }
        }
    }
}
