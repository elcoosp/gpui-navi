use gpui::{AnyElement, App, IntoElement};

/// Suspense boundary for handling async loading states.
pub struct SuspenseBoundary {
    pending_component: Option<AnyElement>,
    pending_ms: u64,
    pending_min_ms: u64,
    children: Vec<AnyElement>,
}

impl SuspenseBoundary {
    pub fn new() -> Self {
        Self {
            pending_component: None,
            pending_ms: 1000,
            pending_min_ms: 500,
            children: Vec::new(),
        }
    }

    pub fn pending_component(mut self, component: impl IntoElement) -> Self {
        self.pending_component = Some(component.into_any_element());
        self
    }

    pub fn pending_ms(mut self, ms: u64) -> Self {
        self.pending_ms = ms;
        self
    }

    pub fn pending_min_ms(mut self, ms: u64) -> Self {
        self.pending_min_ms = ms;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl Default for SuspenseBoundary {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for SuspenseBoundary {
    fn into_any_element(self) -> AnyElement {
        gpui::div().into_any_element()
    }
}
