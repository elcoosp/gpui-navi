use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::ScrollRestoration;

define_route!(
    ScrollRoute,
    path: "/scroll",
    component: ScrollPage,
);

#[derive(Clone, IntoElement)]
struct ScrollPage;

impl RenderOnce for ScrollPage {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let scroll_handle = ScrollHandle::new();
        div()
            .size_full()
            .child(
                div()
                    .id("scroll-container")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .child(
                        div()
                            .child("Scroll Restoration Demo")
                            .children((0..100).map(|i| div().h_8().child(format!("Item {}", i))))
                    )
            )
            .child(ScrollRestoration::new(scroll_handle))
    }
}
