use gpui::prelude::*;
use gpui::{ Window, App, div};
use gpui_kit::component::{v_flex, scroll::ScrollableElement};
use navi_macros::define_route;
use navi_router::components::ScrollRestoration;
use quoin_ui_gpui::NodeElement;

define_route!(
    ScrollRoute,
    path: "/scroll",
    component: ScrollPage,
);

#[derive(Clone, IntoElement)]
struct ScrollPage;

impl RenderOnce for ScrollPage {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // The framework-agnostic restoration node resolves the adapter's
        // shared handle by id (same key as the demo container).
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
                            .children((0..100).map(|i| div().h_8().child(format!("Item {i}"))))
                    ),
            )
            .child(NodeElement(ScrollRestoration::node("navi-scroll-area")))
    }
}
