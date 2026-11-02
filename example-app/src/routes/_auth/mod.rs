use gpui::prelude::*;
use gpui::{ Window, App, div};

use navi_macros::define_route;
use navi_router::components::Outlet;
use quoin_ui_gpui::NodeElement;

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
        div().child("Auth Layout (pathless)").child(NodeElement(Outlet::new().build()))
    }
}
