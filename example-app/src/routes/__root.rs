use gpui::{IntoElement, RenderOnce, Window, App, ParentElement, Styled, div, rgb};
use navi_macros::define_route;
use navi_router::components::{Link, Outlet};
use navi_router::RouterHandle;
use quoin_ui_gpui::NodeElement;

#[derive(Clone, IntoElement)]
struct RootLayout;

impl RenderOnce for RootLayout {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let meta = RouterHandle::current().read().current_meta();
        let title = meta.get("title").and_then(|v| v.as_str()).unwrap_or("Navi App");

        // Update the window title
        window.set_window_title(title);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x001e_1e2e))
            .text_color(rgb(0x00cd_d6f4))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .p_4()
                    .bg(rgb(0x0031_3244))
                    .child(format!("🌐 {title}"))
                    .child(NodeElement(Link::new("/").child("🏠 Home").build()))
                    .child(NodeElement(Link::new("/about").child("ℹ️ About").build()))
                    .child(NodeElement(Link::new("/users").child("👥 Users").build()))
                    .child(NodeElement(Link::new("/users/1").child("User 1").build()))
                    .child(NodeElement(Link::new("/users/2").child("User 2").build()))
                    .child(NodeElement(Link::new("/users/42").child("User 42").build()))
                    .child(NodeElement(Link::new("/settings").child("⚙️ Settings").build()))
                    .child(NodeElement(Link::new("/docs/getting-started").child("📄 Docs").build()))
                    .child(NodeElement(Link::new("/validation-test").child("🧪 Validation").build()))
                    .child(NodeElement(Link::new("/admin").child("🔒 Admin").build()))
                    .child(NodeElement(Link::new("/lifecycle").child("🔄 Lifecycle").build()))
                    .child(NodeElement(Link::new("/blocking").child("🚫 Blocking").build()))
                    .child(NodeElement(Link::new("/scroll").child("📜 Scroll").build()))
                    .child(NodeElement(Link::new("/awaited").child("⏳ Awaited").build()))
                    .child(NodeElement(Link::new("/meta").child("🏷️ Meta").build()))
                    .child(NodeElement(Link::new("/context").child("📦 Context").build()))
                    .child(NodeElement(Link::new("/profile").child("👤 Profile (pathless)").build()))
                    .child(NodeElement(Link::new("/nonexistent").child("❌ 404 Test").build())),
            )
            .child(div().flex_1().p_4().child(NodeElement(Outlet::new().build())))
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
