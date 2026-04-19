use gpui::prelude::*;
use gpui::*;

use navi_macros::define_route;
use navi_router::components::Outlet;

define_route!(
    AuthLayoutRoute,
    path: "/",
    is_layout: true,
    component: AuthLayout,
);

#[derive(Clone, IntoElement)]
struct AuthLayout;
impl RenderOnce for AuthLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Auth Layout (pathless)").child(Outlet::new())
    }
}
