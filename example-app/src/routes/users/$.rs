use gpui::*;
use navi_macros::define_route;

define_route!(
    UsersNotFoundRoute,
    path: "/users/*",
    component: UsersNotFoundPage,
);

#[derive(Clone, IntoElement)]
struct UsersNotFoundPage;

impl RenderOnce for UsersNotFoundPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child("User not found (scoped to /users)")
    }
}
