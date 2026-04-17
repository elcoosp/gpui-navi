use navi_router::RouteDef;
pub mod index;

use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::Outlet;

#[derive(Clone, IntoElement)]
struct UsersLayout;

impl RenderOnce for UsersLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child("👥 Users Section")
            .child(div().flex_1().child(Outlet::new()))
    }
}

define_route!(
    UsersRoute,
    path: "/users",
    is_layout: true,
    component: UsersLayout,
);
