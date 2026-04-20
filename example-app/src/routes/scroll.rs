use gpui::prelude::*;
use gpui::*;
use gpui_component::{v_flex, scroll::ScrollableElement};
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = window
            .use_keyed_state(ElementId::Name("scroll-area".into()), cx, |_, _| {
                ScrollHandle::new()
            })
            .read(cx)
            .clone();

        v_flex()
            .size_full()
            .child(
                v_flex()
                    .id("scroll-area")
                    .flex_1()
                    .min_h_0()
                    .border_2().border_color(gpui::red()) // Visual indicator
                    .overflow_y_scrollbar()
                    .child(
                        v_flex()
                            .child("Scroll Restoration Demo")
                            .children((0..100).map(|i| div().h_8().child(format!("Item {}", i))))
                    ),
            )
            .child(ScrollRestoration::new(scroll_handle))
    }
}
