use gpui::*;
use navi_macros::define_route;
use navi_router::RouterState;

define_route!(
    MetaDemoRoute,
    path: "/meta",
    meta: {
        let mut map = std::collections::HashMap::new();
        map.insert("title".to_string(), serde_json::json!("Meta Demo"));
        map.insert("description".to_string(), serde_json::json!("This route demonstrates per-route meta"));
        map
    },
    component: MetaDemoPage,
);

#[derive(Clone, IntoElement)]
struct MetaDemoPage;

impl RenderOnce for MetaDemoPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let meta = RouterState::global(cx).current_meta();
        div()
            .p_4()
            .child("Meta Demo Page")
            .child(format!("Current meta: {:?}", meta))
    }
}
