use crate::RouterState;
use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Window,
    div,
};
use navi_core::context;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type ComponentConstructor = Arc<dyn Fn(&mut App) -> AnyElement + Send + Sync>;

static REGISTRY: Lazy<Mutex<HashMap<String, ComponentConstructor>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_route_component<F>(route_id: &str, constructor: F)
where
    F: Fn(&mut App) -> AnyElement + Send + Sync + 'static,
{
    REGISTRY
        .lock()
        .unwrap()
        .insert(route_id.to_string(), Arc::new(constructor));
}

/// Context value used to track outlet nesting depth.
#[derive(Clone, Copy)]
pub struct OutletDepth(pub usize);

/// Renders the matched route at the current nesting depth.
#[derive(IntoElement, Default)]
pub struct Outlet {
    children: Vec<AnyElement>,
}

impl Outlet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl ParentElement for Outlet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Outlet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = RouterState::try_global(cx);
        if state.is_none() {
            log::error!("Outlet: RouterState not found in global context");
            return div().child("Router not initialized").into_any_element();
        }
        let state = state.unwrap();

        let window_id = window.window_handle().window_id();
        let depth = context::consume::<OutletDepth>(window_id)
            .map(|d| d.0)
            .unwrap_or(0);

        let (leaf_node_id, constructor_opt) = {
            if let Some((_params, leaf_node)) = state.current_match.as_ref() {
                let ancestors = state.route_tree.ancestors(&leaf_node.id);
                if depth >= ancestors.len() {
                    log::warn!("Outlet depth {} exceeds ancestors length", depth);
                    return div().into_any_element();
                }
                let node = ancestors[depth];
                let constructor = REGISTRY.lock().unwrap().get(&node.id).cloned();
                (node.id.clone(), constructor)
            } else {
                log::warn!("Outlet: no matching route");
                return div().child("404 Not Found").into_any_element();
            }
        };

        if let Some(constructor) = constructor_opt {
            context::provide(window_id, OutletDepth(depth + 1));
            let element = constructor(cx);
            div()
                .id(ElementId::Name(format!("outlet-{}-{}", leaf_node_id.as_str(), depth).into()))
                .child(element)
                .children(self.children)
                .into_any_element()
        } else {
            // Check for 404 component based on not_found_mode
            let not_found_component = match state.not_found_mode {
                crate::NotFoundMode::Root => {
                    REGISTRY.lock().unwrap().get("__not_found_root__").cloned()
                }
                crate::NotFoundMode::Fuzzy => {
                    let ancestors = state.route_tree.ancestors(&leaf_node_id.as_str());
                    ancestors.iter().rev().find_map(|ancestor| {
                        REGISTRY.lock().unwrap().get(&format!("__not_found_{}", ancestor.id)).cloned()
                    })
                }
            };

            if let Some(constructor) = not_found_component {
                context::provide(window_id, OutletDepth(depth + 1));
                let element = constructor(cx);
                div()
                    .id(ElementId::Name(format!("outlet-404-{}", depth).into()))
                    .child(element)
                    .children(self.children)
                    .into_any_element()
            } else {
                log::warn!("No component registered for route: {} and no 404 fallback", leaf_node_id.as_str());
                context::provide(window_id, OutletDepth(depth + 1));
                div()
                    .child(format!("404 - Page not found"))
                    .children(self.children)
                    .into_any_element()
            }
        }
    }
}
