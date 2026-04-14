use gpui::{AnyElement, App, IntoElement, WindowId};
use navi_core::context;

/// Router provider component that initializes the routing context for a window.
pub struct RouterProvider {
    window_id: WindowId,
    children: Vec<AnyElement>,
}

impl RouterProvider {
    pub fn new(window_id: WindowId) -> Self {
        context::init_window(window_id);
        Self {
            window_id,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl IntoElement for RouterProvider {
    fn into_any_element(self) -> AnyElement {
        // In a real implementation, this would render children within a routing context
        gpui::div().into_any_element()
    }
}
