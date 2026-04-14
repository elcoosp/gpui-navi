use crate::RouterState;
use gpui::{App, IntoElement, RenderOnce, ScrollHandle, Window, div};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static SCROLL_POSITIONS: Lazy<Mutex<HashMap<String, f32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Default)]
pub struct ScrollRestoration {
    scroll_handle: Option<ScrollHandle>,
}

impl ScrollRestoration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track_scroll(mut self, handle: ScrollHandle) -> Self {
        self.scroll_handle = Some(handle);
        self
    }

    fn save_position(&self, path: &str) {
        if let Some(handle) = &self.scroll_handle {
            let offset = handle.offset();
            let y: f32 = offset.y.into();
            SCROLL_POSITIONS.lock().unwrap().insert(path.to_string(), y);
        }
    }

    fn restore_position(&self, path: &str) {
        if let Some(handle) = &self.scroll_handle {
            if let Some(&y) = SCROLL_POSITIONS.lock().unwrap().get(path) {
                let mut offset = handle.offset();
                offset.y = gpui::px(y);
                handle.set_offset(offset);
            }
        }
    }
}

impl RenderOnce for ScrollRestoration {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(state) = RouterState::try_global(cx) {
            let path = state.current_location().pathname.clone();
            self.restore_position(&path);
            self.save_position(&path);
        }
        div()
    }
}
