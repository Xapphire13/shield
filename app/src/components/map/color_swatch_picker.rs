use dioxus::prelude::*;
use shield_models::WallColor;

/// Maps a [`WallColor`] to the CSS custom-property suffix used to render it
/// (`--wall-color-{suffix}`). Lives in `app`, not `models`, since `models` is
/// shared with `service` and has no CSS concerns.
pub trait WallColorCssExt {
    fn css_name(&self) -> &'static str;
}

impl WallColorCssExt for WallColor {
    fn css_name(&self) -> &'static str {
        match self {
            WallColor::Slate => "slate",
            WallColor::Clay => "clay",
            WallColor::Moss => "moss",
            WallColor::Amber => "amber",
            WallColor::Sky => "sky",
            WallColor::Rose => "rose",
        }
    }
}

const PALETTE: [(WallColor, &str); 6] = [
    (WallColor::Slate, "Slate"),
    (WallColor::Clay, "Clay"),
    (WallColor::Moss, "Moss"),
    (WallColor::Amber, "Amber"),
    (WallColor::Sky, "Sky"),
    (WallColor::Rose, "Rose"),
];

/// A row of curated color swatches for choosing a wall's display color.
/// Deliberately a small fixed palette (not a full color picker) so wall
/// colors stay visually consistent across a map.
#[component]
pub fn ColorSwatchPicker(value: WallColor, on_change: Callback<WallColor>) -> Element {
    rsx! {
        div { class: "color-swatch-picker",
            for (color , label) in PALETTE {
                button {
                    key: "{color.css_name()}",
                    class: "color-swatch-picker__swatch",
                    "data-color": color.css_name(),
                    "data-selected": value == color,
                    title: "{label}",
                    onclick: move |_| on_change(color),
                }
            }
        }
    }
}
