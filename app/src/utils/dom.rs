use web_sys::{HtmlElement, wasm_bindgen::JsCast, window};

pub fn focus_element(id: &str) -> Result<(), &'static str> {
    window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|el| el.dyn_into::<HtmlElement>().ok())
        .and_then(|input| input.focus().ok())
        .ok_or("Failed to focus element")
}

pub fn reload_page() -> Result<(), &'static str> {
    window()
        .and_then(|w| w.location().reload().ok())
        .ok_or("Failed to reload page")
}

pub fn navigate_to(url: &str) -> Result<(), &'static str> {
    window()
        .and_then(|w| w.location().replace(url).ok())
        .ok_or("Failed to navigate")
}

pub fn get_hostname() -> Result<String, &'static str> {
    window()
        .and_then(|w| w.location().hostname().ok())
        .ok_or("Failed to get hostname")
}
