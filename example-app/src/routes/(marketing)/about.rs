use gpui::prelude::*;
use gpui::*;

use gpui::prelude::*;
use navi_macros::define_route;

define_route!(
    MarketingAboutRoute,
    path: "/about",
    component: MarketingAboutPage,
);

#[derive(Clone, IntoElement)]
struct MarketingAboutPage;
impl RenderOnce for MarketingAboutPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Marketing About Page (inside route group)")
    }
}
