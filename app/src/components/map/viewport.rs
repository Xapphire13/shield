//! The pan/zoom viewport transform for the map canvas, plus the zoom and grid
//! constants that parameterize it. Pure math — no Dioxus or DOM dependency.

/// Minimum / maximum zoom (screen px per logical cm). Panning is intentionally
/// unconstrained, but zoom is clamped to keep the canvas usable.
pub const MIN_ZOOM: f64 = 0.02;
pub const MAX_ZOOM: f64 = 5.0;

/// Multiplier applied per wheel "click" / pinch step.
pub const WHEEL_ZOOM_STEP: f64 = 0.0015;

/// Multiplier applied per click of the `+` / `−` zoom buttons. `+` multiplies
/// the current zoom by this; `−` divides by it. The existing zoom clamp keeps it
/// within `MIN_ZOOM`/`MAX_ZOOM`.
pub const BUTTON_ZOOM_STEP: f64 = 1.2;

/// Target on-screen size (pixels) for a grid square. [`Viewport::grid_spacing_cm`]
/// picks the world-space spacing that keeps squares at or just above this size,
/// so grid density stays legible across the whole `MIN_ZOOM`/`MAX_ZOOM` range
/// instead of turning to mush when zoomed out or ballooning when zoomed in.
pub const TARGET_GRID_SCREEN_PX: f64 = 40.0;

/// Fraction of the canvas the fitted content fills, leaving a margin around it.
pub const FIT_MARGIN: f64 = 0.85;

/// Viewport transform mapping logical world coordinates (centimeters) to screen
/// pixels: `screen = world * zoom + pan`.
///
/// This is the single source of truth for what part of the map is on screen and
/// is deliberately small and self-describing so later rounds can build on it
/// directly — interactive manipulation needs screen->world for hit-testing/dragging,
/// and a minimap needs the world rect currently visible. Both can be derived from
/// these three fields plus the canvas size.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Viewport {
    /// Horizontal pan offset, in screen pixels.
    pub pan_x: f64,
    /// Vertical pan offset, in screen pixels.
    pub pan_y: f64,
    /// Scale factor: screen pixels per logical centimeter.
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        // Start zoomed out enough to comfortably show a ~2000x1500 area with the
        // origin near the top-left of the canvas.
        Self {
            pan_x: 40.0,
            pan_y: 40.0,
            zoom: 0.25,
        }
    }
}

impl Viewport {
    /// Pan by a screen-pixel delta (unconstrained).
    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Zoom by `factor` while keeping the screen point `(sx, sy)` anchored over
    /// the same world coordinate (zoom-to-cursor / zoom-to-pinch-center).
    pub fn zoom_at(&mut self, factor: f64, sx: f64, sy: f64) {
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // World point under the anchor before zooming.
        let wx = (sx - self.pan_x) / self.zoom;
        let wy = (sy - self.pan_y) / self.zoom;
        // Re-derive pan so that world point stays under the anchor.
        self.pan_x = sx - wx * new_zoom;
        self.pan_y = sy - wy * new_zoom;
        self.zoom = new_zoom;
    }

    /// Convert a screen-pixel point (relative to the canvas element) to a world
    /// coordinate in centimeters.
    pub fn screen_to_world(&self, sx: f64, sy: f64) -> (f64, f64) {
        ((sx - self.pan_x) / self.zoom, (sy - self.pan_y) / self.zoom)
    }

    /// Convert a world coordinate (centimeters) to a screen-pixel point
    /// (relative to the canvas element). Inverse of `screen_to_world`.
    pub fn world_to_screen(&self, wx: f64, wy: f64) -> (f64, f64) {
        (wx * self.zoom + self.pan_x, wy * self.zoom + self.pan_y)
    }

    /// SVG transform string for the world-space content group.
    pub fn transform(&self) -> String {
        format!(
            "translate({} {}) scale({})",
            self.pan_x, self.pan_y, self.zoom
        )
    }

    /// World-space grid spacing (centimeters) that keeps grid squares at or
    /// just above [`TARGET_GRID_SCREEN_PX`] on screen at the current zoom, so
    /// the grid steps up/down through "nice" 1-2-5 values (…, 1m, 2m, 5m,
    /// 10m, …) instead of squares shrinking to mush or ballooning as the user
    /// zooms.
    pub fn grid_spacing_cm(&self) -> f64 {
        nice_step_at_least(TARGET_GRID_SCREEN_PX / self.zoom)
    }

    /// A viewport that fits the world-space rectangle `(min_x, min_y, max_x,
    /// max_y)` centered within a canvas of `canvas_w` x `canvas_h` pixels.
    ///
    /// Zoom is chosen so the content fits both axes (with a little headroom via
    /// `FIT_MARGIN`) and clamped to the allowed range; pan then maps the content
    /// center to the canvas center.
    pub fn fit_to_content(bounds: (f64, f64, f64, f64), canvas_w: f64, canvas_h: f64) -> Self {
        let (min_x, min_y, max_x, max_y) = bounds;
        let content_w = (max_x - min_x).max(1.0);
        let content_h = (max_y - min_y).max(1.0);

        let zoom = ((canvas_w / content_w).min(canvas_h / content_h) * FIT_MARGIN)
            .clamp(MIN_ZOOM, MAX_ZOOM);

        let content_cx = (min_x + max_x) / 2.0;
        let content_cy = (min_y + max_y) / 2.0;

        Self {
            pan_x: canvas_w / 2.0 - content_cx * zoom,
            pan_y: canvas_h / 2.0 - content_cy * zoom,
            zoom,
        }
    }
}

/// Smallest value from the repeating 1-2-5 sequence (…, 1, 2, 5, 10, 20, 50,
/// 100, …) that is `>= min`. `min <= 0.0` degenerates to `1.0` since the
/// sequence has no smallest positive member.
///
/// The trailing `10.0` candidate (in addition to `1.0`/`2.0`/`5.0` scaled by
/// `base`) guards against floating-point log/pow round-trip error landing
/// `base` one power of ten low (e.g. for `min` exactly on a power of ten).
fn nice_step_at_least(min: f64) -> f64 {
    if min <= 0.0 {
        return 1.0;
    }
    let base = 10f64.powf(min.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|mult| base * mult)
        .find(|&candidate| candidate >= min)
        .expect("10.0 * base always satisfies base * 10.0 >= min")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_step_at_least_picks_smallest_covering_1_2_5_value() {
        assert_eq!(nice_step_at_least(1.0), 1.0);
        assert_eq!(nice_step_at_least(1.5), 2.0);
        assert_eq!(nice_step_at_least(2.0), 2.0);
        assert_eq!(nice_step_at_least(3.0), 5.0);
        assert_eq!(nice_step_at_least(50.0), 50.0);
        assert_eq!(nice_step_at_least(51.0), 100.0);
        assert_eq!(nice_step_at_least(8.0), 10.0);
        assert_eq!(nice_step_at_least(2000.0), 2000.0);
    }

    #[test]
    fn nice_step_at_least_handles_non_positive_input() {
        assert_eq!(nice_step_at_least(0.0), 1.0);
        assert_eq!(nice_step_at_least(-5.0), 1.0);
    }

    #[test]
    fn grid_spacing_cm_keeps_squares_at_or_above_target_size_across_zoom_range() {
        let mut zoom = MIN_ZOOM;
        while zoom <= MAX_ZOOM {
            let viewport = Viewport {
                pan_x: 0.0,
                pan_y: 0.0,
                zoom,
            };
            let spacing = viewport.grid_spacing_cm();
            let on_screen_px = spacing * zoom;
            assert!(
                on_screen_px >= TARGET_GRID_SCREEN_PX,
                "zoom {zoom}: spacing {spacing} cm -> {on_screen_px} px, below target"
            );
            zoom *= 1.7;
        }
    }

    #[test]
    fn zoom_at_keeps_anchor_over_same_world_point() {
        let mut viewport = Viewport::default();
        let (anchor_x, anchor_y) = (123.0, 456.0);
        let world_before = viewport.screen_to_world(anchor_x, anchor_y);
        viewport.zoom_at(1.5, anchor_x, anchor_y);
        let world_after = viewport.screen_to_world(anchor_x, anchor_y);
        assert!((world_before.0 - world_after.0).abs() < 1e-9);
        assert!((world_before.1 - world_after.1).abs() < 1e-9);
    }

    #[test]
    fn world_to_screen_inverts_screen_to_world() {
        let viewport = Viewport {
            pan_x: -30.0,
            pan_y: 70.0,
            zoom: 0.4,
        };
        let (wx, wy) = viewport.screen_to_world(200.0, 100.0);
        let (sx, sy) = viewport.world_to_screen(wx, wy);
        assert!((sx - 200.0).abs() < 1e-9);
        assert!((sy - 100.0).abs() < 1e-9);
    }

    #[test]
    fn fit_to_content_centers_content_in_canvas() {
        let bounds = (0.0, 0.0, 1000.0, 500.0);
        let viewport = Viewport::fit_to_content(bounds, 800.0, 600.0);
        // The content center should land on the canvas center.
        let (sx, sy) = viewport.world_to_screen(500.0, 250.0);
        assert!((sx - 400.0).abs() < 1e-9);
        assert!((sy - 300.0).abs() < 1e-9);
        // Zoom fits the tighter axis with the fit margin applied.
        assert!((viewport.zoom - (800.0 / 1000.0) * FIT_MARGIN).abs() < 1e-9);
    }
}
