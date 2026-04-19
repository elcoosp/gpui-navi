use crate::RouterState;
use gpui::{AnyElement, App, IntoElement, RenderOnce, Window};

pub struct SuspenseBoundary {
    fallback: Box<dyn Fn() -> AnyElement>,
}

impl SuspenseBoundary {
    pub fn new(fallback: impl Fn() -> AnyElement + 'static) -> Self {
        Self {
            fallback: Box::new(fallback),
        }
    }
}

impl RenderOnce for SuspenseBoundary {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = RouterState::global(cx);
        let has_pending = state.has_pending_loader();
        let _pending_ms = state.default_pending_ms;
        let _pending_min_ms = state.default_pending_min_ms;

        if has_pending {
            (self.fallback)()
        } else {
            crate::components::Outlet::new().into_any_element()
        }
    }
}
