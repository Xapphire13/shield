//! Disable the browser's native page pinch-zoom, which fights the map's own
//! pinch/scroll zoom. Two mechanisms are needed because browsers disagree on
//! how pinches reach the page:
//!
//! - Android browsers honor the viewport meta's `maximum-scale` /
//!   `user-scalable` fields, so those are rewritten on the tag dioxus ships.
//!   Safari ignores them (iOS deliberately since iOS 10; desktop ignores the
//!   viewport meta entirely), as do desktop Chrome/Firefox.
//! - Safari (mobile and desktop trackpad) implements pinches as proprietary
//!   `gesture*` events whose default action zooms the page; cancelling them
//!   is the only way to opt out. Components that want pinch behavior (the
//!   map) listen for these events themselves — `preventDefault` doesn't stop
//!   propagation.

use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// See the module docs. Call once at the app root; the cancel closure is held
/// in component state so the listeners stay valid for the app's lifetime.
pub fn use_disable_page_zoom() {
    use_hook(|| {
        let document = web_sys::window().and_then(|w| w.document());

        if let Some(meta) = document
            .as_ref()
            .and_then(|d| d.query_selector("meta[name='viewport']").ok().flatten())
        {
            let _ = meta.set_attribute(
                "content",
                "width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no",
            );
        }

        let cancel = Closure::<dyn FnMut(web_sys::Event)>::new(|evt: web_sys::Event| {
            evt.prevent_default();
        });
        if let Some(document) = &document {
            for kind in ["gesturestart", "gesturechange", "gestureend"] {
                let _ = document
                    .add_event_listener_with_callback(kind, cancel.as_ref().unchecked_ref());
            }
        }

        std::rc::Rc::new(cancel)
    });
}
