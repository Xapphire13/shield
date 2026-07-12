//! Track a DOM element's bounding rect (viewport-relative origin + size) via a
//! `ResizeObserver`, looked up by element id.

use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// Look up an element in the DOM by its id.
fn element_by_id(id: &str) -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(id)
}

/// Read an element's bounding rect as `(left, top, width, height)` in viewport
/// pixels, or `None` if it isn't in the DOM yet.
pub fn element_rect(id: &str) -> Option<(f64, f64, f64, f64)> {
    let rect = element_by_id(id)?.get_bounding_client_rect();
    Some((rect.left(), rect.top(), rect.width(), rect.height()))
}

/// Run `f` after the browser has applied layout for the current frame, using a
/// double `requestAnimationFrame` (one frame to apply layout, one to be safe).
/// Each callback is one-shot, so `Closure::once_into_js` is used to hand it to
/// the browser without manual lifetime bookkeeping.
pub fn after_next_layout(f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let inner = Closure::once_into_js(f);
    let outer = Closure::once_into_js(move || {
        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(inner.unchecked_ref());
        }
    });
    let _ = window.request_animation_frame(outer.unchecked_ref());
}

/// The tracked rect: both signals update together whenever the observed
/// element's size changes (including once after the initial layout).
pub struct UseElementRectResult {
    /// The element's viewport-relative top-left, in pixels.
    pub origin: Signal<(f64, f64)>,
    /// The element's width and height, in pixels.
    pub size: Signal<(f64, f64)>,
}

/// Observe the element with id `element_id` with a `ResizeObserver` rather
/// than a one-shot mount read. A mount-time read can run before the browser's
/// first layout pass on a fresh / deep-link load, measuring a not-yet-laid-out
/// box; the observer instead fires once *after* layout (fixing that case) and
/// again on every size change, so it also subsumes a window-resize listener
/// and is the single source of truth for both origin and size.
pub fn use_element_rect(element_id: &'static str) -> UseElementRectResult {
    let mut origin = use_signal(|| (0.0_f64, 0.0_f64));
    let mut size = use_signal(|| (0.0_f64, 0.0_f64));

    // The observer + its callback closure are held in component state so they
    // stay alive for the component's lifetime.
    let _observer = use_hook(move || {
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some((left, top, width, height)) = element_rect(element_id) {
                origin.set((left, top));
                size.set((width, height));
            }
        });

        // The element may not be in the DOM on the very first effect tick;
        // retry on the next layout frame if so.
        let observer = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()).ok();
        if let Some(observer) = &observer {
            if let Some(element) = element_by_id(element_id) {
                observer.observe(&element);
            } else {
                let observer = observer.clone();
                after_next_layout(move || {
                    if let Some(element) = element_by_id(element_id) {
                        observer.observe(&element);
                    }
                });
            }
        }

        // Keep both alive for the component's lifetime: dropping the closure
        // would invalidate the observer's callback, and dropping the observer
        // would stop notifications. `Rc` makes the stored state `Clone` (which
        // `use_hook` requires) without cloning the non-`Clone` closure.
        std::rc::Rc::new((observer, callback))
    });

    UseElementRectResult { origin, size }
}
