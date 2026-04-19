use gpui::*;
use navi_macros::define_route;

define_route!(
    GlobalNotFoundRoute,
    path: "/*",
    component: GlobalNotFoundPage,
);

#[derive(Clone, IntoElement)]
struct GlobalNotFoundPage;

impl RenderOnce for GlobalNotFoundPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child("404 - Page Not Found (Global)")
    }
}
