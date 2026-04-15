use crate::RouterState;
use gpui::{App, IntoElement, RenderOnce, Window, div};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static SCROLL_POSITIONS: Lazy<Mutex<HashMap<String, f32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Default)]
pub struct ScrollRestoration;

impl ScrollRestoration {
    pub fn new() -> Self {
        Self
    }
}

impl RenderOnce for ScrollRestoration {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(state) = RouterState::try_global(cx) {
            let path = state.current_location().pathname.clone();
            // Restore scroll position (placeholder)
            if let Some(&_y) = SCROLL_POSITIONS.lock().unwrap().get(&path) {
                // In a real implementation, you'd set the scroll offset
                // window.set_scroll_offset(...);
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
