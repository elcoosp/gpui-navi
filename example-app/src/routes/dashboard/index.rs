use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct OverviewData {
    pub stats: Vec<String>,
}

define_route!(
    DashboardIndexRoute,
    path: "/dashboard",
    is_index: true,
    data: OverviewData,
    loader: |_params: (), executor: gpui::BackgroundExecutor| async move {
        executor.timer(Duration::from_millis(600)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(OverviewData {
            stats: vec![
                "Users: 1,234".to_string(),
                "Revenue: $5,678".to_string(),
                "Orders: 42".to_string(),
            ],
        }))
    },
    component: DashboardOverview,
);

#[derive(Clone, IntoElement)]
struct DashboardOverview;

impl RenderOnce for DashboardOverview {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        use navi_macros::use_loader_data;
        let data = use_loader_data!(DashboardIndexRoute);
        div()
            .p_4()
            .child("📈 Overview")
            .children(data.map(|d| {
                d.stats.iter().map(|s| div().child(s.clone())).collect::<Vec<_>>()
            }).unwrap_or_default())
    }
}
