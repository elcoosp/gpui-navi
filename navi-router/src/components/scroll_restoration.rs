use gpui::{AnyElement, App, IntoElement};
use std::collections::HashMap;

/// Scroll restoration component that saves and restores scroll positions.
pub struct ScrollRestoration {
    scroll_positions: HashMap<String, f32>,
}

impl ScrollRestoration {
    pub fn new() -> Self {
        Self {
            scroll_positions: HashMap::new(),
        }
    }

    pub fn save_position(&mut self, path: String, position: f32) {
        self.scroll_positions.insert(path, position);
    }

    pub fn get_position(&self, path: &str) -> Option<f32> {
        self.scroll_positions.get(path).copied()
    }
}

impl Default for ScrollRestoration {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for ScrollRestoration {
    fn into_any_element(self) -> AnyElement {
        gpui::div().into_any_element()
    }
}
