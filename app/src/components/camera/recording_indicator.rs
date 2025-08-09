use dioxus::prelude::*;

#[component]
pub fn RecordingIndicator(is_recording: bool) -> Element {
    // let fill = if is_recording { "red" } else { "#B6B6B6" };

    rsx! {
        svg {
            width: "16px",
            height: "16px",
            class: "recording-indicator",
            "data-recording": is_recording,

            // Outer circle
            defs {
                mask { id: "recording-indicator-mask",

                    rect {
                        x: 0,
                        y: 0,
                        width: "16",
                        height: "16",
                        fill: "white",
                    }

                    circle {
                        cx: "8",
                        cy: "8",
                        r: "7",
                        fill: "black",
                    }
                }
            }
            circle {
                cx: "8",
                cy: "8",
                r: "8",
                fill: "var(--fill)",
                mask: "url(#recording-indicator-mask)",
            }

            // Inner Circle
            circle {
                cx: "8",
                cy: "8",
                r: "5",
                fill: "var(--fill)",
            }
        }
    }
}
