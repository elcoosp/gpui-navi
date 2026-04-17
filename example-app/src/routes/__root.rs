use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::{Link, Outlet, SuspenseBoundary};

#[derive(Clone, IntoElement)]
struct RootLayout;

impl RenderOnce for RootLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
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
                    .child(Link::new("/").child("🏠 Home"))
                    .child(Link::new("/about").child("ℹ️ About"))
                    .child(Link::new("/users").child("👥 Users"))
                    .child(Link::new("/dashboard").child("📊 Dashboard"))
                    .child(Link::new("/settings").child("⚙️ Settings"))
                    .child(Link::new("/docs/getting-started").child("📄 Docs"))
                    .child(Link::new("/validation-test").child("🧪 Validation")),
            )
            .child(
                SuspenseBoundary::new(|| {
                    div()
                        .p_2()
                        .bg(rgb(0x2563eb))
                        .text_color(gpui::white())
                        .w_full()
                        .text_center()
                        .child("⏳ Loading...")
                        .into_any_element()
                })
                .with_child(div().flex_1().p_4().child(Outlet::new()))
            )
    }
}

define_route!(
    RootRoute,
    path: "/",
    is_layout: true,
    component: RootLayout,
);
