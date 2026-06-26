use dioxus::prelude::*;

/// Placeholder map view. For now this renders an empty SVG "canvas" that will
/// later host the property map — buildings, landscape features, camera
/// locations and field-of-view indicators.
#[component]
pub fn MapView() -> Element {
    rsx! {
        div { class: "primary-view map-view",
            svg {
                class: "map-canvas",
                xmlns: "http://www.w3.org/2000/svg",

                // Faint grid to hint at the editable drawing surface.
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
            }

            div { class: "map-view__hint", "Property map coming soon" }
        }
    }
}
