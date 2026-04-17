use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct AnalyticsData {
    pub chart_data: Vec<u32>,
}

define_route!(
    DashboardAnalyticsRoute,
    path: "/dashboard/analytics",
    data: AnalyticsData,
    loader: |_params: (), executor: gpui::BackgroundExecutor| async move {
        executor.timer(Duration::from_millis(800)).await;
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
        use navi_macros::use_loader_data;
        let data = use_loader_data!(DashboardAnalyticsRoute);
        div()
            .p_4()
            .child("📊 Analytics")
            .children(data.map(|d| {
                d.chart_data.iter().map(|v| div().child(format!("Value: {}", v))).collect::<Vec<_>>()
            }).unwrap_or_default())
    }
}
