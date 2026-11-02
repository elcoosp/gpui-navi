use gpui::prelude::*;
use gpui::{ Window, App, div};
use navi_macros::define_route;
use navi_router::components::Awaited;
use quoin_ui_gpui::NodeElement;
use std::time::Duration;

define_route!(
    AwaitedDemoRoute,
    path: "/awaited",
    data: String,
    loader: |()| async move {
        navi_router::sleep(Duration::from_secs(2)).await;
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
            .child(NodeElement(
                Awaited::<AwaitedDemoRoute>::new()
                    .fallback(|| quoin_ui::Node::div().child("Loading...").build())
                    .child(|data: String| {
                        quoin_ui::Node::div().child(format!("Data: {data}")).build()
                    })
                    .build(),
            ))
    }
}
