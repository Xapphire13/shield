use dioxus::prelude::*;

/// Shared top bar for the primary views: a fixed-height bar with a centered
/// title flanked by optional side controls.
///
/// The layout is a three-zone grid (start | title | actions) whose side columns
/// are equal width, so the title stays centered on the whole bar regardless of
/// how wide the side content is. Both side slots default to empty, so a view
/// that only needs a title can render `TopBar { title: "…" }`; the start zone is
/// still rendered (empty) in that case to keep the two side columns balanced.
#[component]
pub fn TopBar(
    title: String,
    #[props(default = rsx! {})] start: Element,
    #[props(default = rsx! {})] actions: Element,
) -> Element {
    rsx! {
        div { class: "top-bar",
            div { class: "top-bar__start", {start} }

            span { class: "top-bar__title", {title} }

            div { class: "top-bar__actions", {actions} }
        }
    }
}
