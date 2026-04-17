use crate::RouterState;
use gpui::div;
use gpui::{AnyElement, App, IntoElement, RenderOnce, Window};
#[derive(IntoElement)]
pub struct SuspenseBoundary {
    fallback: Box<dyn Fn() -> AnyElement>,
    child: Option<AnyElement>,
}

impl SuspenseBoundary {
    pub fn new(fallback: impl Fn() -> AnyElement + 'static) -> Self {
        Self {
            fallback: Box::new(fallback),
            child: None,
        }
    }

    pub fn with_child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl RenderOnce for SuspenseBoundary {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_pending = RouterState::try_global(cx)
            .map(|s| s.has_pending_loader())
            .unwrap_or(false);

        if has_pending {
            (self.fallback)()
        } else {
            self.child.unwrap_or_else(|| div().into_any_element())
        }
    }
}
