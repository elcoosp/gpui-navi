use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::{BeforeLoadResult, redirect, components::Outlet};

define_route!(
    AdminRoute,
    path: "/admin",
    is_layout: true,
    before_load: |_ctx| async move {
        // Simulate auth check - toggle to test
        let is_authenticated = false; // change to true to allow access
        if !is_authenticated {
            BeforeLoadResult::Redirect(redirect("/login"))
        } else {
            BeforeLoadResult::Ok
        }
    },
    component: AdminLayout,
);

#[derive(Clone, IntoElement)]
struct AdminLayout;

impl RenderOnce for AdminLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .p_4()
            .child("Admin Area (protected by beforeLoad)")
            .child(Outlet::new())
    }
}
