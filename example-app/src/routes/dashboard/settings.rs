use gpui::prelude::*;
use gpui::*;
use navi_macros::{define_route, use_loader_data};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsData {
    pub theme: String,
}

define_route!(
    DashboardSettingsRoute,
    path: "/dashboard/settings",
    data: SettingsData,
    loader: |_: (), executor: gpui::BackgroundExecutor| async move {
        log::info!("[Settings] Loader started");
        executor.timer(Duration::from_millis(300)).await;
        log::info!("[Settings] Loader completed");
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(SettingsData {
            theme: "dark".to_string(),
        }))
    },
    component: DashboardSettings,
);

#[derive(Clone, IntoElement)]
struct DashboardSettings;

impl RenderOnce for DashboardSettings {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = use_loader_data!(DashboardSettingsRoute);
        let theme = data.map(|d| d.theme.clone()).unwrap_or_else(|| "⏳ Loading...".to_string());
        div().p_4().child("⚙️ Settings").child(format!("Theme: {}", theme))
    }
}
