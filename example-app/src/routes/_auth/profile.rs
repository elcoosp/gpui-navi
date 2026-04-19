use gpui::prelude::*;
use gpui::*;

use navi_macros::define_route;

define_route!(
    ProfileRoute,
    path: "/profile",
    component: ProfilePage,
);

#[derive(Clone, IntoElement)]
struct ProfilePage;
impl RenderOnce for ProfilePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Profile Page (inside pathless auth layout)")
    }
}
