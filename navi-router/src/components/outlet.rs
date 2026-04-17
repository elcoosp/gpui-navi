// navi-router/src/components/outlet.rs
use crate::RouterState;
use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Window,
    div,
};
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
/// Depth is automatically inferred from the context and incremented for children.
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = RouterState::try_global(cx);
        if state.is_none() {
            log::error!("Outlet: RouterState not found in global context");
            return div().child("Router not initialized").into_any_element();
        }
        let state = state.unwrap();

        // Determine current depth from GPUI context (default 0)
        let depth = cx.consume::<OutletDepth>().map(|d| d.0).unwrap_or(0);

        // Extract the node id and constructor
        let (node_id, constructor_opt) = {
            if let Some((_params, leaf_node)) = state.current_match.as_ref() {
                let ancestors = state.route_tree.ancestors(&leaf_node.id);
                log::debug!(
                    "Outlet ancestors for {}: {:?}",
                    leaf_node.id,
                    ancestors.iter().map(|n| &n.id).collect::<Vec<_>>()
                );
                if depth >= ancestors.len() {
                    log::warn!(
                        "Outlet depth {} exceeds ancestors length {} for route {}",
                        depth,
                        ancestors.len(),
                        leaf_node.id
                    );
                    return div().into_any_element();
                }
                let node = ancestors[depth];
                log::debug!("Outlet (depth {}) rendering route: {}", depth, node.id);

                let constructor = REGISTRY.lock().unwrap().get(&node.id).cloned();
                (node.id.clone(), constructor)
            } else {
                log::warn!("Outlet: no matching route");
                return div().child("404 Not Found").into_any_element();
            }
        };

        if let Some(constructor) = constructor_opt {
            // Provide incremented depth for child outlets
            cx.provide(OutletDepth(depth + 1));
            let element = constructor(cx);
            div()
                .id(ElementId::Name(
                    format!("outlet-{}-{}", node_id, depth).into(),
                ))
                .child(element)
                .children(self.children)
                .into_any_element()
        } else {
            log::warn!("No component registered for route: {}", node_id);
            cx.provide(OutletDepth(depth + 1));
            div()
                .child(format!("No component for route: {}", node_id))
                .children(self.children)
                .into_any_element()
        }
    }
}
