use navi_router::RouteDef;
#[cfg(feature = "validify")]
use gpui::prelude::*;
#[cfg(feature = "validify")]
use gpui::*;
#[cfg(feature = "validify")]
use navi_macros::{define_route, use_search};
#[cfg(feature = "validify")]
use navi_router::components::Link;
#[cfg(feature = "validify")]
use serde::Deserialize;
#[cfg(feature = "validify")]
use validify::Validate;

#[cfg(feature = "validify")]
#[derive(Debug, Deserialize, Validate, Clone, Default)]
pub struct ValidifySearch {
    #[validate(range(min = 1, max = 100))]
    pub page: Option<u32>,
    #[validate(length(min = 1, max = 10))]
    pub sort: Option<String>,
}

#[cfg(feature = "validify")]
define_route!(
    ValidifyTestRoute,
    path: "/validation-test/validify",
    search: ValidifySearch,
    component: ValidifyTestPage,
);

#[cfg(feature = "validify")]
#[derive(Clone, IntoElement)]
pub struct ValidifyTestPage;

#[cfg(feature = "validify")]
impl RenderOnce for ValidifyTestPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let raw = use_search!(ValidifyTestRoute);
        let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
        div()
            .p_4()
            .child("Validify Test Page")
            .child(format!("Page: {:?}, Sort: {:?}", validated.page, validated.sort))
            .child(Link::new("/validation-test/validify?page=50&sort=desc").child("Valid params"))
            .child(Link::new("/validation-test/validify?page=999&sort=invalid").child("Invalid params"))
            .child(Link::new("/validation-test").child("Back"))
    }
}
