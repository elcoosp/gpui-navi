// navi-router/src/components/outlet.rs
use crate::RouterState;
use gpui::{AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, Window, div};
use navi_core::context;
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
    depth: Option<usize>,
    children: Vec<AnyElement>,
}

impl Outlet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = Some(depth);
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

        // Determine the outlet depth: use explicit depth or read from context
        let depth = self.depth.unwrap_or_else(|| {
            context::consume::<OutletDepth>(window.window_handle().window_id())
                .map(|d| d.0)
                .unwrap_or(0)
        });

        // Extract the node id and the constructor *before* releasing the immutable borrow on state.
        let (node_id, constructor_opt) = {
            if let Some((_params, leaf_node)) = state.current_match.as_ref() {
                let ancestors = state.route_tree.ancestors(&leaf_node.id);
                if depth >= ancestors.len() {
                    log::warn!(
                        "Outlet depth {} exceeds ancestors length {}",
                        depth,
                        ancestors.len()
                    );
                    return div()
                        .child("No matching route at this depth")
                        .into_any_element();
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
        // `state` borrow is now released; we can mutate `cx` safely.

        // Provide the next depth for child outlets
        let window_id = window.window_handle().window_id();
        context::provide(window_id, OutletDepth(depth + 1));

        if let Some(constructor) = constructor_opt {
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
            div()
                .child(format!("No component for route: {}", node_id))
                .children(self.children)
                .into_any_element()
        }
    }
}

/// Context value used to track outlet nesting depth.
#[derive(Clone, Copy)]
struct OutletDepth(usize);
