use dioxus::prelude::*;
use shield_models::{FieldOfView, MapCamera, Point};

use crate::components::map::map_camera::MapCameraMarker;

/// Minimum / maximum zoom (screen px per logical cm). Panning is intentionally
/// unconstrained (see issue #11), but zoom is clamped to keep the canvas usable.
const MIN_ZOOM: f64 = 0.02;
const MAX_ZOOM: f64 = 5.0;

/// Multiplier applied per wheel "click" / pinch step.
const WHEEL_ZOOM_STEP: f64 = 0.0015;

/// Viewport transform mapping logical world coordinates (centimeters) to screen
/// pixels: `screen = world * zoom + pan`.
///
/// This is the single source of truth for what part of the map is on screen and
/// is deliberately small and self-describing so later rounds can build on it
/// directly — #6 (manipulation) needs screen->world for hit-testing/dragging and
/// #8 (minimap) needs the world rect currently visible. Both can be derived from
/// these three fields plus the canvas size.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Viewport {
    /// Horizontal pan offset, in screen pixels.
    pan_x: f64,
    /// Vertical pan offset, in screen pixels.
    pan_y: f64,
    /// Scale factor: screen pixels per logical centimeter.
    zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        // Start zoomed out enough to comfortably show the mock ~2000x1500 area
        // with the origin near the top-left of the canvas.
        Self {
            pan_x: 40.0,
            pan_y: 40.0,
            zoom: 0.25,
        }
    }
}

impl Viewport {
    /// Pan by a screen-pixel delta (unconstrained).
    fn pan_by(&mut self, dx: f64, dy: f64) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Zoom by `factor` while keeping the screen point `(sx, sy)` anchored over
    /// the same world coordinate (zoom-to-cursor / zoom-to-pinch-center).
    fn zoom_at(&mut self, factor: f64, sx: f64, sy: f64) {
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // World point under the anchor before zooming.
        let wx = (sx - self.pan_x) / self.zoom;
        let wy = (sy - self.pan_y) / self.zoom;
        // Re-derive pan so that world point stays under the anchor.
        self.pan_x = sx - wx * new_zoom;
        self.pan_y = sy - wy * new_zoom;
        self.zoom = new_zoom;
    }

    /// SVG transform string for the world-space content group.
    fn transform(&self) -> String {
        format!(
            "translate({} {}) scale({})",
            self.pan_x, self.pan_y, self.zoom
        )
    }
}

/// Active gesture being tracked across pointer/touch events.
#[derive(Clone, Copy, PartialEq)]
enum Gesture {
    None,
    /// One-pointer pan; stores the last screen position seen.
    Pan {
        last_x: f64,
        last_y: f64,
    },
    /// Two-finger pinch; stores the last finger distance (the midpoint is
    /// recomputed each move and used as the zoom anchor).
    Pinch {
        last_distance: f64,
    },
}

/// Mock cameras spread across a ~2000x1500 cm area with varied directions and
/// ranges, so the canvas renders something meaningful.
///
/// PLACEHOLDER: this is hardcoded mock data. Round 3 (#6) wires this view to the
/// real `use_map` hook (PR #3) for live map data; remove this then.
fn mock_cameras() -> Vec<MapCamera> {
    vec![
        MapCamera {
            camera_id: "mock-1".to_string(),
            position: Point { x: 300, y: 300 },
            fov: FieldOfView {
                direction_deg: 135, // aimed SE
                angle_deg: 70,
                range: 700,
            },
        },
        MapCamera {
            camera_id: "mock-2".to_string(),
            position: Point { x: 1700, y: 350 },
            fov: FieldOfView {
                direction_deg: 225, // aimed SW
                angle_deg: 90,
                range: 600,
            },
        },
        MapCamera {
            camera_id: "mock-3".to_string(),
            position: Point { x: 1000, y: 1200 },
            fov: FieldOfView {
                direction_deg: 0, // aimed North
                angle_deg: 50,
                range: 900,
            },
        },
        MapCamera {
            camera_id: "mock-4".to_string(),
            position: Point { x: 250, y: 1100 },
            fov: FieldOfView {
                direction_deg: 60, // aimed ENE
                angle_deg: 110,
                range: 500,
            },
        },
    ]
}

/// Euclidean distance between two points.
fn distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

/// Read-only, pan/zoom-able map canvas rendering cameras and their FOV cones
/// from mock data. The viewport foundation later rounds build on.
#[component]
pub fn MapView() -> Element {
    let mut viewport = use_signal(Viewport::default);
    let mut gesture = use_signal(|| Gesture::None);

    let cameras = use_signal(mock_cameras);

    let transform = viewport.read().transform();

    rsx! {
        div { class: "primary-view map-view",
            svg {
                class: "map-canvas",
                xmlns: "http://www.w3.org/2000/svg",
                // Touch-action none lets us own panning/pinching instead of the
                // browser scrolling/zooming the page.
                style: "touch-action: none;",

                // --- Wheel zoom (desktop) ---
                onwheel: move |evt| {
                    let delta = evt.data().delta().strip_units().y;
                    let coords = evt.data().client_coordinates();
                    // Scrolling up (negative delta) zooms in.
                    let factor = (-delta * WHEEL_ZOOM_STEP).exp();
                    viewport.write().zoom_at(factor, coords.x, coords.y);
                },

                // --- Pointer pan (mouse / single-finger) ---
                onpointerdown: move |evt| {
                    let coords = evt.data().client_coordinates();
                    gesture
                        .set(Gesture::Pan {
                            last_x: coords.x,
                            last_y: coords.y,
                        });
                },
                onpointermove: move |evt| {
                    let current = *gesture.read();
                    if let Gesture::Pan { last_x, last_y } = current {
                        let coords = evt.data().client_coordinates();
                        let dx = coords.x - last_x;
                        let dy = coords.y - last_y;
                        viewport.write().pan_by(dx, dy);
                        gesture
                            .set(Gesture::Pan {
                                last_x: coords.x,
                                last_y: coords.y,
                            });
                    }
                },
                onpointerup: move |_| {
                    if matches!(*gesture.read(), Gesture::Pan { .. }) {
                        gesture.set(Gesture::None);
                    }
                },
                onpointercancel: move |_| {
                    if matches!(*gesture.read(), Gesture::Pan { .. }) {
                        gesture.set(Gesture::None);
                    }
                },

                // --- Touch pinch zoom ---
                ontouchstart: move |evt| {
                    let touches = evt.data().touches();
                    if touches.len() == 2 {
                        let a = touches[0].client_coordinates();
                        let b = touches[1].client_coordinates();
                        gesture
                            .set(Gesture::Pinch {
                                last_distance: distance(a.x, a.y, b.x, b.y),
                            });
                    }
                },
                ontouchmove: move |evt| {
                    let touches = evt.data().touches();
                    let current = *gesture.read();
                    if touches.len() == 2
                        && let Gesture::Pinch { last_distance, .. } = current
                    {
                        let a = touches[0].client_coordinates();
                        let b = touches[1].client_coordinates();
                        let dist = distance(a.x, a.y, b.x, b.y);
                        let cx = (a.x + b.x) / 2.0;
                        let cy = (a.y + b.y) / 2.0;
                        if last_distance > 0.0 {
                            let factor = dist / last_distance;
                            viewport.write().zoom_at(factor, cx, cy);
                        }
                        gesture.set(Gesture::Pinch { last_distance: dist });
                    }
                },
                ontouchend: move |_| {
                    if matches!(*gesture.read(), Gesture::Pinch { .. }) {
                        gesture.set(Gesture::None);
                    }
                },

                // Faint grid backdrop (screen-fixed) to hint at the surface.
                defs {
                    pattern {
                        id: "map-grid",
                        width: "32",
                        height: "32",
                        "patternUnits": "userSpaceOnUse",
                        path {
                            d: "M 32 0 L 0 0 0 32",
                            fill: "none",
                            stroke: "#2a2f3e",
                            "stroke-width": "1",
                        }
                    }
                }
                rect {
                    width: "100%",
                    height: "100%",
                    fill: "url(#map-grid)",
                }

                // World-space content: pan + zoom applied here so cameras stay
                // in logical cm coordinates.
                g { transform: "{transform}",
                    for camera in cameras.read().iter().cloned() {
                        MapCameraMarker { key: "{camera.camera_id}", camera }
                    }
                }
            }
        }
    }
}
