use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;

#[derive(Clone, IntoElement)]
struct AboutPage;

impl RenderOnce for AboutPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .child("ℹ️ About Navi Router")
            .child("A powerful file‑based router for GPUI with loaders, suspense, and devtools.")
    }
}

define_route!(
    AboutRoute,
    path: "/about",
    meta: {
        let mut map = std::collections::HashMap::new();
        map.insert("title".to_string(), serde_json::json!("About Navi"));
        map
    },
    component: AboutPage,
);
