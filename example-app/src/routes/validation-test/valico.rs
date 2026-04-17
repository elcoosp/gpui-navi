#[cfg(feature = "valico")]
use gpui::prelude::*;
#[cfg(feature = "valico")]
use gpui::*;
#[cfg(feature = "valico")]
use navi_macros::{define_route, use_search};
#[cfg(feature = "valico")]
use navi_router::RouteDef;
#[cfg(feature = "valico")]
use navi_router::components::Link;
#[cfg(feature = "valico")]
use serde::Deserialize;
#[cfg(feature = "valico")]
use schemars::JsonSchema;

#[cfg(feature = "valico")]
#[derive(Debug, Deserialize, Default, Clone, JsonSchema)]
pub struct ValicoSearch {
    pub page: Option<u32>,
    pub sort: Option<String>,
}

#[cfg(feature = "valico")]
impl ValicoSearch {
    fn validate(&self) -> Result<(), String> {
        if let Some(p) = self.page {
            if !(1..=100).contains(&p) {
                return Err("page out of range".into());
            }
        }
        if let Some(s) = &self.sort {
            if s.len() > 10 {
                return Err("sort too long".into());
            }
        }
        Ok(())
    }
}

#[cfg(feature = "valico")]
define_route!(
    ValicoTestRoute,
    path: "/validation-test/valico",
    search: ValicoSearch,
    component: ValicoTestPage,
);

#[cfg(feature = "valico")]
#[derive(Clone, IntoElement)]
pub struct ValicoTestPage;

#[cfg(feature = "valico")]
impl RenderOnce for ValicoTestPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let raw = use_search!(ValicoTestRoute);
        let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
        div()
            .p_4()
            .child("Valico Test Page")
            .child(format!("Page: {:?}, Sort: {:?}", validated.page, validated.sort))
            .child(Link::new("/validation-test/valico?page=50&sort=desc").child("Valid params"))
            .child(Link::new("/validation-test/valico?page=999&sort=invalid").child("Invalid params"))
            .child(Link::new("/validation-test").child("Back"))
    }
}
