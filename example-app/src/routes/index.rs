use gpui::prelude::*;
use gpui::*;
use gpui_component::scroll::ScrollableElement;
use navi_router::RouteDef;
use navi_macros::define_route;

#[derive(Clone, IntoElement)]
struct HomePage;

impl RenderOnce for HomePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .overflow_y_scrollbar()
            .gap_6()
            .p_8()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child("🏠 Welcome to Navi Router!"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xa6adc8))
                    .child("This demo showcases nested layouts, dynamic routes, loaders, and more."),
            )
            .child(
                div()
                    .max_w(px(600.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .text_color(rgb(0xbac2de))
                    .child("Lorem ipsum dolor sit amet...")
            )
    }
}

define_route!(
    IndexRoute,
    path: "/",
    is_index: true,
    component: HomePage,
);
