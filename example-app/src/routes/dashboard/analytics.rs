use gpui::prelude::*;
use gpui::*;
use navi_macros::{define_route, use_loader_data};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct AnalyticsData {
    pub chart_data: Vec<u32>,
}

define_route!(
    DashboardAnalyticsRoute,
    path: "/dashboard/analytics",
    data: AnalyticsData,
    loader: |_: (), executor: gpui::BackgroundExecutor| async move {
        log::info!("[Analytics] Loader started");
        executor.timer(Duration::from_millis(800)).await;
        log::info!("[Analytics] Loader completed, returning data");
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(AnalyticsData {
            chart_data: vec![10, 25, 15, 30, 20],
        }))
    },
    component: DashboardAnalytics,
);

#[derive(Clone, IntoElement)]
struct DashboardAnalytics;

impl RenderOnce for DashboardAnalytics {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = use_loader_data!(DashboardAnalyticsRoute);
        log::debug!("[Analytics] Render: data = {:?}", data);
        match data {
            Some(d) => {
                div()
                    .p_4()
                    .child("📊 Analytics")
                    .children(d.chart_data.into_iter().map(|v| div().child(format!("Value: {}", v))))
            }
            None => {
                div().p_4().child("📊 Analytics").child("⏳ Loading analytics...")
            }
        }
    }
}
