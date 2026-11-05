//! Navi demo on Leptos (CSR) — the shared `NaviDemo` shell mounted to the
//! document body through quoin's Leptos emitter.

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    use leptos::prelude::*;
    use navi_demo_core::NaviDemo;

    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap_or_default();
    leptos::mount::mount_to_body(|| view! { <NaviDemo /> });
}
