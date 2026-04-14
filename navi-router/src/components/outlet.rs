use gpui::{AnyElement, App, IntoElement};

/// Outlet component that renders the matched child route.
pub struct Outlet {
    children: Vec<AnyElement>,
}

impl Outlet {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }
}

impl Default for Outlet {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for Outlet {
    fn into_any_element(self) -> AnyElement {
        gpui::div().into_any_element()
    }
}
