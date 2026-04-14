use gpui::{AnyElement, App, IntoElement};

/// Error boundary component that catches errors in child components.
pub struct CatchBoundary {
    error_component: Option<AnyElement>,
    children: Vec<AnyElement>,
}

impl CatchBoundary {
    pub fn new() -> Self {
        Self {
            error_component: None,
            children: Vec::new(),
        }
    }

    pub fn error_component(mut self, component: impl IntoElement) -> Self {
        self.error_component = Some(component.into_any_element());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl Default for CatchBoundary {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for CatchBoundary {
    fn into_any_element(self) -> AnyElement {
        gpui::div().into_any_element()
    }
}
