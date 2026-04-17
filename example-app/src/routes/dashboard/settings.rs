use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsData {
    pub theme: String,
}

define_route!(
    DashboardSettingsRoute,
    path: "/dashboard/settings",
    data: SettingsData,
    loader: |_params: (), executor: gpui::BackgroundExecutor| async move {
        executor.timer(Duration::from_millis(300)).await;
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
        use navi_macros::use_loader_data;
        let data = use_loader_data!(DashboardSettingsRoute);
        div()
            .p_4()
            .child("⚙️ Settings")
            .child(data.map(|d| format!("Theme: {}", d.theme)).unwrap_or_default())
    }
}
