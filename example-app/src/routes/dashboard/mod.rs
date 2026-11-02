use gpui::prelude::*;
use gpui::{ Window, App, div, rgb, FontWeight};
use navi_macros::{define_route, use_loader_data};
use navi_router::components::Outlet;
use std::time::Duration;
use quoin_ui_gpui::NodeElement;

#[derive(Clone, Debug, PartialEq)]
pub struct DashboardData {
    pub message: String,
}

define_route!(
    DashboardRoute,
    path: "/dashboard",
    is_layout: true,
    data: DashboardData,
    loader: |(): ()| async move {
        log::info!("[Dashboard Layout] Loader started");
        navi_router::sleep(Duration::from_millis(400)).await;
        log::info!("[Dashboard Layout] Loader completed");
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(DashboardData {
            message: "Dashboard layout loaded".to_string(),
        }))
    },
    component: DashboardLayout,
);

#[derive(Clone, IntoElement)]
struct DashboardLayout;

impl RenderOnce for DashboardLayout {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let data = use_loader_data!(DashboardRoute);
        let message = match data {
            Some(d) => d.message.clone(),
            None => "⏳ Loading layout...".to_string(),
        };
        div()
            .p_4()
            .bg(rgb(0x001a_1a2e))
            .text_color(rgb(0x00ee_eeee))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child("📊 Dashboard Layout"),
            )
            .child(div().text_sm().child(message))
            .child(div().mt_4().child(NodeElement(Outlet::new().build())))
    }
}
