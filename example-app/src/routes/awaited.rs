use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::Awaited;
use std::time::Duration;

define_route!(
    AwaitedDemoRoute,
    path: "/awaited",
    data: String,
    loader: |_, executor: gpui::BackgroundExecutor| async move {
        executor.timer(Duration::from_secs(2)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new("Loaded data!".to_string()))
    },
    component: AwaitedDemoPage,
);

#[derive(Clone, IntoElement)]
struct AwaitedDemoPage;

impl RenderOnce for AwaitedDemoPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .child("Awaited Demo")
            .child(
                Awaited::<AwaitedDemoRoute>::new()
                    .fallback(|| div().child("Loading...").into_any_element())
                    .child(|data: String| div().child(format!("Data: {}", data)).into_any_element())
            )
    }
}
