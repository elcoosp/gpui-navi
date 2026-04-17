# validator.rs
cat > src/routes/validation-test/validator.rs << 'EOF'
#[cfg(feature = "validator")]
use gpui::prelude::*;
#[cfg(feature = "validator")]
use gpui::*;
#[cfg(feature = "validator")]
use navi_macros::{define_route, use_search};
#[cfg(feature = "validator")]
use navi_router::RouteDef;
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
EOF

# garde.rs
cat > src/routes/validation-test/garde.rs << 'EOF'
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
EOF

# validify.rs
cat > src/routes/validation-test/validify.rs << 'EOF'
#[cfg(feature = "validify")]
use gpui::prelude::*;
#[cfg(feature = "validify")]
use gpui::*;
#[cfg(feature = "validify")]
use navi_macros::{define_route, use_search};
#[cfg(feature = "validify")]
use navi_router::RouteDef;
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
EOF

# valico.rs
cat > src/routes/validation-test/valico.rs << 'EOF'
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
EOF
