use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::Location;

define_route!(
    LifecycleRoute,
    path: "/lifecycle",
    on_enter: |loc: &Location| {
        log::info!("Entered lifecycle route at {}", loc.pathname);
    },
    on_leave: |loc: &Location| {
        log::info!("Left lifecycle route from {}", loc.pathname);
    },
    component: LifecyclePage,
);

#[derive(Clone, IntoElement)]
struct LifecyclePage;

impl RenderOnce for LifecyclePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Lifecycle Demo - check console logs")
    }
}
