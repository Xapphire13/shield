use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::fi_icons::FiX;
use shield_models::Camera;

use crate::components::ui::{ButtonColor, IconButton};
use crate::utils::RecordingModeExtensions;

/// Nominal popover width (px) used for the horizontal anchor/clamp math. The
/// element's CSS width matches this so the computed left/caret offsets line up
/// with what actually renders.
const POPOVER_WIDTH: f64 = 240.0;

/// Estimated popover height (px) used only to decide whether there is room above
/// the marker before flipping below. A generous estimate errs toward flipping,
/// which is the safe direction (the popover never overflows the top).
const POPOVER_HEIGHT_ESTIMATE: f64 = 160.0;

/// Gap (px) between the marker anchor and the popover edge (leaves room for the
/// caret).
const ANCHOR_GAP: f64 = 16.0;

/// Minimum inset (px) kept between the popover and the viewport edges when
/// clamping horizontally.
const VIEWPORT_MARGIN: f64 = 8.0;

/// Minimum inset (px) the caret keeps from the popover's own left/right edges so
/// it never detaches from the rounded corners.
const CARET_EDGE_MARGIN: f64 = 14.0;

/// Compact, read-only info popover for a camera shown on the map in view mode,
/// anchored next to its marker.
///
/// Mirrors the edit-mode [`CameraInspector`](super::camera_inspector) surface
/// styling but exposes no edit controls. It is positioned with `position: fixed`
/// at the marker's on-screen point (supplied by the host, recomputed from the
/// live viewport so the popover follows the marker on pan/zoom). It defaults to
/// sitting *above* the marker with a downward caret, flips *below* (upward caret)
/// when there is no room above, and is clamped horizontally to the viewport while
/// the caret stays aligned to the marker's x.
///
/// A `pinned` popover (opened by a tap) shows a close button; a hover-only
/// popover hides on mouse-leave and needs none. The hover case is gated to
/// hover-capable devices in CSS.
#[component]
pub fn CameraInfo(
    /// The resolved camera for the marker, or `None` when its placed reference is
    /// an orphan (the underlying camera was deleted).
    camera: Option<Camera>,
    /// Marker x on screen (viewport px): the caret points here.
    anchor_x: f64,
    /// Marker y on screen (viewport px): the popover sits above or below this.
    anchor_y: f64,
    /// Whether the popover was pinned by a tap (vs. shown on hover). Pinned shows
    /// a close button and always renders; hover-only is gated to hover devices.
    pinned: bool,
    /// Dismiss the popover (pinned/tap case only).
    on_close: Callback,
) -> Element {
    let orphaned = camera.is_none();
    let title = camera
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Unknown camera".to_string());

    let (viewport_w, viewport_h) = viewport_size();

    // Horizontal: center on the marker, then clamp so the box stays on screen.
    let unclamped_left = anchor_x - POPOVER_WIDTH / 2.0;
    let max_left = (viewport_w - POPOVER_WIDTH - VIEWPORT_MARGIN).max(VIEWPORT_MARGIN);
    let left = unclamped_left.clamp(VIEWPORT_MARGIN, max_left);
    // Keep the caret over the marker even after the box is clamped.
    let caret_left = (anchor_x - left).clamp(CARET_EDGE_MARGIN, POPOVER_WIDTH - CARET_EDGE_MARGIN);

    // Vertical: prefer above (caret down). Flip below (caret up) when the box
    // wouldn't clear the top of the viewport.
    let room_above = anchor_y - ANCHOR_GAP - POPOVER_HEIGHT_ESTIMATE >= VIEWPORT_MARGIN;
    let place_below = !room_above;
    let style = if place_below {
        let top = anchor_y + ANCHOR_GAP;
        format!(
            "position: fixed; left: {left}px; top: {top}px; width: {POPOVER_WIDTH}px; --caret-left: {caret_left}px;"
        )
    } else {
        // `bottom` is measured from the viewport bottom, so the popover's bottom
        // edge lands `ANCHOR_GAP` above the marker.
        let bottom = viewport_h - (anchor_y - ANCHOR_GAP);
        format!(
            "position: fixed; left: {left}px; bottom: {bottom}px; width: {POPOVER_WIDTH}px; --caret-left: {caret_left}px;"
        )
    };
    let placement = if place_below { "below" } else { "above" };

    rsx! {
        div {
            class: "camera-info",
            "data-placement": placement,
            "data-pinned": pinned,
            style: "{style}",
            div { class: "camera-info__header",
                div {
                    class: "camera-info__title",
                    "data-orphaned": orphaned,
                    "{title}"
                }
                if pinned {
                    IconButton {
                        icon: rsx! {
                            Icon { width: 18, height: 18, icon: FiX }
                        },
                        color: ButtonColor::Default,
                        on_press: move |_| on_close(()),
                    }
                }
            }

            if let Some(camera) = camera {
                div { class: "camera-info__rows",
                    div { class: "camera-info__row",
                        span { class: "camera-info__label", "Status" }
                        span {
                            class: "camera-info__value",
                            "data-recording": camera.is_recording,
                            if camera.is_recording {
                                "Recording"
                            } else {
                                "Not recording"
                            }
                        }
                    }
                    div { class: "camera-info__row",
                        span { class: "camera-info__label", "Recording mode" }
                        span { class: "camera-info__value",
                            "{camera.recording_settings.mode.display_name()}"
                        }
                    }
                }

                if !camera.tags.is_empty() {
                    div { class: "camera-info__tags",
                        for tag in camera.tags.iter() {
                            span { key: "{tag}", class: "camera-info__tag", "{tag}" }
                        }
                    }
                }
            } else {
                div { class: "camera-info__note",
                    "This camera no longer exists."
                }
            }

            // Caret pointing at the marker; its side and horizontal offset are
            // driven by `data-placement` and the `--caret-left` custom property.
            div { class: "camera-info__caret" }
        }
    }
}

/// Current browser viewport size in CSS px, or `(0, 0)` when unavailable. Used
/// for clamping the popover within the visible area.
fn viewport_size() -> (f64, f64) {
    let Some(window) = web_sys::window() else {
        return (0.0, 0.0);
    };
    let w = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let h = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    (w, h)
}
