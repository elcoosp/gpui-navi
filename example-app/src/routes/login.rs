use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;

define_route!(
    LoginRoute,
    path: "/login",
    component: LoginPage,
);

#[derive(Clone, IntoElement)]
struct LoginPage;

impl RenderOnce for LoginPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Login Page (redirect target)")
    }
}
