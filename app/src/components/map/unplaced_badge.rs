use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::LdTriangleAlert;

/// Floating lower-left badge warning that some cameras have not been placed on
/// the map. The host renders it only when `count > 0`, so it never shows a zero.
///
/// It pairs a warning-triangle glyph with the count and, on hover-capable
/// devices, reveals a fuller tooltip via CSS (gated to `@media (hover: hover)`,
/// matching the project's hover convention). A native `title` attribute carries
/// the same message as a simple accessible fallback everywhere.
#[component]
pub fn UnplacedBadge(
    /// Number of cameras that exist but are not yet placed on the map.
    count: usize,
) -> Element {
    let plural = if count == 1 { "camera" } else { "cameras" };
    let message = format!("{count} {plural} not on the map");

    rsx! {
        div {
            class: "map-unplaced-badge",
            title: "{message}",
            Icon { width: 16, height: 16, icon: LdTriangleAlert }
            span { class: "map-unplaced-badge__count", "{count}" }
            span { class: "map-unplaced-badge__tooltip", "{message}" }
        }
    }
}
