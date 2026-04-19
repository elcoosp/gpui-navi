use crate::RouterState;
use gpui::{AnyElement, App, IntoElement, RenderOnce, Window};

/// Suspense boundary that shows a fallback while loaders are pending.
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
        let state = RouterState::try_global(cx);
        let has_pending = state.map(|s| s.has_pending_loader()).unwrap_or(false);
        let _pending_ms = state.map(|s| s.default_pending_ms).unwrap_or(1000);
        let _pending_min_ms = state.map(|s| s.default_pending_min_ms).unwrap_or(500);

        // TODO: implement timing logic with element state to show fallback only after pending_ms
        // and keep it visible for at least pending_min_ms.
        if has_pending {
            (self.fallback)()
        } else {
            crate::components::Outlet::new().into_any_element()
        }
    }
}
