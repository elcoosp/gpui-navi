use crate::RouterState;
use gpui::{App, IntoElement, RenderOnce, Window, div};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static SCROLL_POSITIONS: Lazy<Mutex<HashMap<String, f32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct ScrollRestoration;

impl ScrollRestoration {
    pub fn new() -> Self {
        Self
    }

    pub fn save(path: &str, y: f32) {
        SCROLL_POSITIONS.lock().unwrap().insert(path.to_string(), y);
    }

    pub fn get(path: &str) -> Option<f32> {
        SCROLL_POSITIONS.lock().unwrap().get(path).copied()
    }
}

impl RenderOnce for ScrollRestoration {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(state) = RouterState::try_global(cx) {
            let path = state.current_location().pathname.clone();
            // In a real implementation, you'd apply the saved scroll position to the scrollable element.
            // Here we just log it.
            if let Some(y) = Self::get(&path) {
                log::debug!("Restoring scroll position for {}: {}", path, y);
            }
        }
        div()
    }
}

impl IntoElement for ScrollRestoration {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
