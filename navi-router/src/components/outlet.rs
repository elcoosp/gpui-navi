use crate::RouterState;
use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Window,
    div,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

type ComponentConstructor = Box<dyn Fn(&mut App) -> AnyElement + Send + Sync>;

static REGISTRY: Lazy<Mutex<HashMap<String, ComponentConstructor>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_route_component<F>(route_id: &str, constructor: F)
where
    F: Fn(&mut App) -> AnyElement + Send + Sync + 'static,
{
    REGISTRY
        .lock()
        .unwrap()
        .insert(route_id.to_string(), Box::new(constructor));
}

#[derive(IntoElement, Default)]
pub struct Outlet {
    children: Vec<AnyElement>,
}

impl Outlet {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for Outlet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Outlet {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = RouterState::try_global(cx);
        if state.is_none() {
            log::error!("Outlet: RouterState not found in global context");
            return div().child("Router not initialized").into_any_element();
        }
        let state = state.unwrap();
        let current_match = state.current_match.as_ref();

        if let Some((_params, node)) = current_match {
            log::debug!("Outlet rendering route: {}", node.id);

            // 🔑 创建基于当前 location（包含 search）的唯一 ID
            let location = state.current_location();
            let search_hash = seahash::hash(location.search.to_string().as_bytes());
            let key = ElementId::Name(format!("outlet-{}-{}", node.id, search_hash).into());

            if let Some(constructor) = REGISTRY.lock().unwrap().get(&node.id) {
                let element = constructor(cx);
                return div()
                    .id(key) // 动态 ID 破坏缓存
                    .child(element)
                    .children(self.children)
                    .into_any_element();
            } else {
                log::warn!("No component registered for route: {}", node.id);
                return div()
                    .child(format!("No component for route: {}", node.id))
                    .children(self.children)
                    .into_any_element();
            }
        } else {
            log::warn!("Outlet: no matching route");
            div().child("404 Not Found").into_any_element()
        }
    }
}
