use gpui::*;
use navi_macros::define_route;
use navi_router::route_tree::RouteContextArgs;

define_route!(
    ContextDemoRoute,
    path: "/context",
    data: String,
    loader: |_, _| async move {
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new("Loader data".to_string()))
    },
    context: |args: RouteContextArgs| {
        let mut map = serde_json::Map::new();
        map.insert("from_context".to_string(), serde_json::json!("Hello from context!"));
        if let Some(data) = args.loader_data {
            if let Some(s) = data.0.downcast_ref::<std::sync::Arc<String>>() {
                map.insert("loader_data".to_string(), serde_json::json!(**s));
            }
        }
        serde_json::Value::Object(map)
    },
    component: ContextDemoPage,
);

#[derive(Clone, IntoElement)]
struct ContextDemoPage;

impl RenderOnce for ContextDemoPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let context = navi_macros::use_route_context!(ContextDemoRoute);
        div()
            .p_4()
            .child("Context Demo Page")
            .child(format!("Route context: {:?}", context))
    }
}
