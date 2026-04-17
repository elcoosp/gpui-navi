#[cfg(feature = "garde")]
use gpui::prelude::*;
#[cfg(feature = "garde")]
use gpui::*;
#[cfg(feature = "garde")]
use navi_macros::{define_route, use_search};
#[cfg(feature = "garde")]
use navi_router::RouteDef;
#[cfg(feature = "garde")]
use navi_router::components::Link;
#[cfg(feature = "garde")]
use serde::Deserialize;
#[cfg(feature = "garde")]
use garde::Validate as GardeValidate;

#[cfg(feature = "garde")]
#[derive(Debug, Deserialize, Clone, GardeValidate, Default)]
#[garde(allow_unvalidated)]
pub struct GardeSearch {
    #[garde(range(min = 1, max = 100))]
    pub page: Option<u32>,
    #[garde(length(min = 1, max = 10))]
    pub sort: Option<String>,
}

#[cfg(feature = "garde")]
define_route!(
    GardeTestRoute,
    path: "/validation-test/garde",
    search: GardeSearch,
    component: GardeTestPage,
);

#[cfg(feature = "garde")]
#[derive(Clone, IntoElement)]
pub struct GardeTestPage;

#[cfg(feature = "garde")]
impl RenderOnce for GardeTestPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let raw = use_search!(GardeTestRoute);
        let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
        div()
            .p_4()
            .child("Garde Test Page")
            .child(format!("Page: {:?}, Sort: {:?}", validated.page, validated.sort))
            .child(Link::new("/validation-test/garde?page=50&sort=desc").child("Valid params"))
            .child(Link::new("/validation-test/garde?page=999&sort=invalid").child("Invalid params"))
            .child(Link::new("/validation-test").child("Back"))
    }
}
