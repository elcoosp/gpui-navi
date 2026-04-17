use gpui::prelude::*;
use gpui::*;
use gpui_component::scroll::ScrollableElement;
use navi_macros::define_route;
use navi_router::RouterState;
use navi_router::components::{Link, Outlet, PreloadType, SuspenseBoundary};

#[derive(Clone, IntoElement)]
struct RootLayout;

impl RenderOnce for RootLayout {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = RouterState::global(cx).main_scroll_handle();

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
                    .child(
                        Link::new("/dashboard")
                            .preload(PreloadType::Intent)
                            .child("📊 Dashboard"),
                    )
                    .child(Link::new("/settings").child("⚙️ Settings"))
                    .child(Link::new("/docs/getting-started").child("📄 Docs"))
                    .child(Link::new("/validation-test").child("🧪 Validation")),
            )
            .child(
                SuspenseBoundary::new(|| {
                    div()
                        .w_full()
                        .h(px(3.0))
                        .bg(rgb(0x2563eb))
                        .into_any_element()
                })
                .with_child(
                    div()
                        .id("main-scroll-container")
                        .flex_1()
                        .p_4()
                        .track_scroll(&scroll_handle)
                        .overflow_y_scrollbar()
                        .child(Outlet::new()),
                ),
            )
    }
}

define_route!(
    RootRoute,
    path: "/",
    is_layout: true,
    component: RootLayout,
);
