use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;

define_route!(
    AdminDashboardRoute,
    path: "/admin/dashboard",
    data: String,
    loader: |_params, _executor| async move {
        // Simulate conditional not found
        let should_404 = true; // toggle to test
        if should_404 {
            Err("Not found".to_string())
        } else {
            Ok(std::sync::Arc::new("Dashboard data".to_string()))
        }
    },
    component: DashboardPage,
);

#[derive(Clone, IntoElement)]
struct DashboardPage;

impl RenderOnce for DashboardPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = navi_macros::use_loader_data!(AdminDashboardRoute);
        match data {
            Some(d) => div().child(format!("Dashboard: {}", d)),
            None => div().child("Loading or not found..."),
        }
    }
}
