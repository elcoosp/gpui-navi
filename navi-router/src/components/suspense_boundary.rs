use crate::RouterState;
use gpui::{AnyElement, App, IntoElement, RenderOnce, Window};
use navi_core::context;

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
        let has_pending = RouterState::try_global(cx)
            .map(|s| s.has_pending_loader())
            .unwrap_or(false);

        if has_pending {
            (self.fallback)()
        } else {
            crate::components::Outlet::new().into_any_element()
        }
    }
}
