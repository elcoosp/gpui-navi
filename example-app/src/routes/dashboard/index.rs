use gpui::prelude::*;
use gpui::*;
use navi_macros::{define_route, use_loader_data};
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
    loader: |_: (), executor: gpui::BackgroundExecutor| async move {
        log::info!("[Overview] Loader started");
        executor.timer(Duration::from_millis(600)).await;
        log::info!("[Overview] Loader completed");
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
        let data = use_loader_data!(DashboardIndexRoute);
        let stats = data
            .map(|d| d.stats.clone())
            .unwrap_or_else(|| vec!["⏳ Loading overview...".to_string()]);
        div()
            .p_4()
            .child("📈 Overview")
            .children(stats.into_iter().map(|s| div().child(s)))
    }
}
