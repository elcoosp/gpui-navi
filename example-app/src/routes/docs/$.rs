use gpui::prelude::*;
use gpui::{ Window, App, div};
use navi_macros::define_route;
use navi_router::RouterHandle;

#[derive(Clone, IntoElement)]
struct DocsSplatPage;

impl RenderOnce for DocsSplatPage {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        let path = RouterHandle::try_current()
            .map(|h| h.read().current_location().pathname)
            .unwrap_or_default();
        let slug = path.strip_prefix("/docs/").unwrap_or("unknown");
        div()
            .child(format!("📄 Documentation: {slug}"))
            .child("This is a catch‑all splat route.")
    }
}

define_route!(
    DocsRoute,
    path: "/docs/$",
    component: DocsSplatPage,
);
