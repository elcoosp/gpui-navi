use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::{Link, Outlet};
use navi_router::RouterState;

#[derive(Clone, IntoElement)]
struct RootLayout;

impl RenderOnce for RootLayout {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let meta = RouterState::global(cx).current_meta();
        let title = meta.get("title").and_then(|v| v.as_str()).unwrap_or("Navi App");
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_4()
                    .bg(rgb(0x313244))
                    .child(format!("🌐 {}", title))
                    .child(Link::new("/").child("🏠 Home"))
                    .child(Link::new("/about").child("ℹ️ About"))
                    .child(Link::new("/users").child("👥 Users"))
                    .child(Link::new("/users/1").child("User 1"))
                    .child(Link::new("/users/2").child("User 2"))
                    .child(Link::new("/settings").child("⚙️ Settings"))
                    .child(Link::new("/docs/getting-started").child("📄 Docs (splat)"))
                    .child(Link::new("/validation-test").child("🧪 Validation Tests"))
                    .child(Link::new("/admin").child("🔒 Admin"))
                    .child(Link::new("/lifecycle").child("🔄 Lifecycle"))
                    .child(Link::new("/blocking").child("🚫 Blocking"))
            )
            .child(div().flex_1().p_4().child(Outlet::new()))
    }
}

define_route!(
    RootRoute,
    path: "/",
    is_layout: true,
    meta: {
        let mut map = std::collections::HashMap::new();
        map.insert("title".to_string(), serde_json::json!("Navi Demo"));
        map
    },
    component: RootLayout,
);
