//! Bridge Safari's proprietary `gesture*` events to a pinch-zoom callback.
//!
//! Safari is the only browser that delivers trackpad pinches this way — no
//! `wheel` event (Chrome/Firefox synthesize a ctrl+wheel) and no `touch*`
//! events fire — so `onwheel` / `ontouch*` handlers never see them. iOS
//! Safari *also* fires these alongside the real touch events for two-finger
//! pinches, so callers that handle touch pinches themselves must de-duplicate
//! (see the caller in `MapView`).
//!
//! `use_disable_page_zoom` cancels the events' default page zoom app-wide;
//! `preventDefault` doesn't stop propagation, so these listeners still run.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// `GestureEvent` has no web-sys binding (WebKit-only), so its fields are
/// read reflectively.
fn event_f64(evt: &web_sys::Event, key: &str) -> Option<f64> {
    js_sys::Reflect::get(evt.as_ref(), &key.into())
        .ok()?
        .as_f64()
}

/// Call `on_pinch(factor, client_x, client_y)` for each step of a Safari
/// pinch: `factor` is the zoom ratio since the previous step (>1 spreading,
/// <1 contracting) and the coordinates are the pinch midpoint in client
/// (viewport) pixels.
///
/// Listens at the document level so registration doesn't depend on any
/// element being mounted; the closures are held in component state so the
/// listeners stay valid for the component's lifetime.
pub fn use_safari_pinch(mut on_pinch: impl FnMut(f64, f64, f64) + 'static) {
    use_hook(move || {
        // The event's `scale` is cumulative since gesturestart, so each step
        // applies the ratio against the previous event's value.
        let last_scale = Rc::new(Cell::new(1.0_f64));

        let on_start = {
            let last_scale = last_scale.clone();
            Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
                last_scale.set(event_f64(&evt, "scale").unwrap_or(1.0));
            })
        };

        let on_change = {
            let last_scale = last_scale.clone();
            Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
                let Some(scale) = event_f64(&evt, "scale") else {
                    return;
                };
                let last = last_scale.get();
                last_scale.set(scale);
                let (Some(cx), Some(cy)) = (event_f64(&evt, "clientX"), event_f64(&evt, "clientY"))
                else {
                    return;
                };
                if last > 0.0 {
                    on_pinch(scale / last, cx, cy);
                }
            })
        };

        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            let _ = document.add_event_listener_with_callback(
                "gesturestart",
                on_start.as_ref().unchecked_ref(),
            );
            let _ = document.add_event_listener_with_callback(
                "gesturechange",
                on_change.as_ref().unchecked_ref(),
            );
        }

        Rc::new((on_start, on_change))
    });
}
