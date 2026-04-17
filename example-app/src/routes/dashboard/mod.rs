use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::Outlet;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DashboardParams;

#[derive(Clone, Debug, PartialEq)]
pub struct DashboardData {
    pub message: String,
}

define_route!(
    DashboardRoute,
    path: "/dashboard",
    is_layout: true,
    params: DashboardParams,
    data: DashboardData,
    loader: |_params: DashboardParams, executor: gpui::BackgroundExecutor| async move {
        executor.timer(Duration::from_millis(400)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(DashboardData {
            message: "Dashboard layout loaded".to_string(),
        }))
    },
    component: DashboardLayout,
);

#[derive(Clone, IntoElement)]
struct DashboardLayout;

impl RenderOnce for DashboardLayout {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        use navi_macros::use_loader_data;
        let data = use_loader_data!(DashboardRoute);
        div()
            .p_4()
            .bg(rgb(0x1a1a2e))
            .text_color(rgb(0xeeeeee))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child("📊 Dashboard Layout"),
            )
            .child(div().text_sm().child(data.map(|d| d.message.clone()).unwrap_or_default()))
            .child(div().mt_4().child(Outlet::new()))
    }
}
