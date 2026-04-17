use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::RouteDef;
use navi_router::RouterState;

#[derive(Clone, IntoElement)]
struct DocsSplatPage;

impl RenderOnce for DocsSplatPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let path = RouterState::try_global(cx)
            .map(|s| s.current_location().pathname)
            .unwrap_or_default();
        let slug = path.strip_prefix("/docs/").unwrap_or("unknown");
        div()
            .child(format!("📄 Documentation: {}", slug))
            .child("This is a catch‑all splat route.")
    }
}

define_route!(
    DocsRoute,
    path: "/docs/$",
    component: DocsSplatPage,
);
