use navi_router::RouteDef;

#[cfg(feature = "validator")]
use gpui::prelude::*;
#[cfg(feature = "validator")]
use gpui::*;
#[cfg(feature = "validator")]
use navi_macros::{define_route, use_search};
#[cfg(feature = "validator")]
use navi_router::components::Link;
#[cfg(feature = "validator")]
use serde::Deserialize;
#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "validator")]
#[derive(Debug, Deserialize, Validate, Clone, Default)]
pub struct ValidatorSearch {
    #[validate(range(min = 1, max = 100))]
    pub page: Option<u32>,
    #[validate(length(min = 1, max = 10))]
    pub sort: Option<String>,
}

#[cfg(feature = "validator")]
define_route!(
    ValidatorTestRoute,
    path: "/validation-test/validator",
    search: ValidatorSearch,
    component: ValidatorTestPage,
);

#[cfg(feature = "validator")]
#[derive(Clone, IntoElement)]
pub struct ValidatorTestPage;

#[cfg(feature = "validator")]
impl RenderOnce for ValidatorTestPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let raw = use_search!(ValidatorTestRoute);
        let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
        div()
            .p_4()
            .child("Validator Test Page")
            .child(format!("Page: {:?}, Sort: {:?}", validated.page, validated.sort))
            .child(Link::new("/validation-test/validator?page=50&sort=desc").child("Valid params"))
            .child(Link::new("/validation-test/validator?page=999&sort=invalid").child("Invalid params"))
            .child(Link::new("/validation-test").child("Back"))
    }
}
