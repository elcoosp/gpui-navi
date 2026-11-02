//! Route outlet: renders the registered component for the matched route.

use crate::RouterHandle;
use crate::state::NotFoundMode;
use quoin_ui::Node;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

type NodeConstructor = Arc<dyn Fn() -> Node + Send + Sync>;

static REGISTRY: LazyLock<Mutex<HashMap<String, NodeConstructor>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Register the node constructor rendered by an [`Outlet`] for a route id.
///
/// # Panics
/// Panics if the global route registry mutex is poisoned.
pub fn register_route_component<F>(route_id: &str, constructor: F)
where
    F: Fn() -> Node + Send + Sync + 'static,
{
    REGISTRY
        .lock()
        .expect("route registry lock")
        .insert(route_id.to_string(), Arc::new(constructor));
}

/// The registered constructor for `route_id`, if any.
///
/// # Panics
/// Panics if the route-component registry lock is poisoned.
#[must_use]
pub fn route_component(route_id: &str) -> Option<NodeConstructor> {
    REGISTRY
        .lock()
        .expect("route registry lock")
        .get(route_id)
        .cloned()
}

#[derive(Clone, Copy)]
pub struct OutletDepth(pub usize);

/// Renders the matched route's component at the current nesting depth.
#[derive(Clone, Default)]
pub struct Outlet {
    children: Vec<Node>,
}

impl Outlet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn child(mut self, child: impl Into<quoin_ui::NodeChild>) -> Self {
        self.children.push(child.into().0);
        self
    }

    /// Builds the outlet's node tree.
    #[must_use]
    pub fn build(self) -> Node {
        let Some(handle) = RouterHandle::try_current() else {
            log::error!("Outlet: RouterState not found");
            return Node::div()
                .styles([
                    quoin_ui::StyleCmd::SizeFull,
                    quoin_ui::StyleCmd::Flex,
                    quoin_ui::StyleCmd::FlexCol,
                ])
                .child("Router not initialized")
                .build();
        };

        let depth = {
            let state = handle.read();
            let window_key = state.window_key;
            navi_core::context::consume::<OutletDepth>(window_key).map_or(0, |d| d.0)
        };

        if depth > 20 {
            log::error!("Outlet depth {depth} exceeded limit - possible cycle in route tree");
            return Node::div()
                .styles([
                    quoin_ui::StyleCmd::SizeFull,
                    quoin_ui::StyleCmd::Flex,
                    quoin_ui::StyleCmd::FlexCol,
                ])
                .child("Error: Outlet depth limit exceeded")
                .build();
        }

        let state = handle.read();
        let Some((_params, leaf_node)) = state.current_match.as_ref() else {
            log::warn!("Outlet: no matching route");
            return Node::div()
                .styles([
                    quoin_ui::StyleCmd::SizeFull,
                    quoin_ui::StyleCmd::Flex,
                    quoin_ui::StyleCmd::FlexCol,
                ])
                .child("404 Not Found")
                .build();
        };

        let ancestors = state.route_tree.ancestors(&leaf_node.id);
        if depth >= ancestors.len() {
            log::warn!("Outlet depth {depth} exceeds ancestors length");
            return Node::div()
                .styles([
                    quoin_ui::StyleCmd::SizeFull,
                    quoin_ui::StyleCmd::Flex,
                    quoin_ui::StyleCmd::FlexCol,
                ])
                .build();
        }
        let node = &ancestors[depth];
        let leaf_node_id = node.id.clone();
        let constructor = route_component(&node.id);

        let not_found_constructor = if constructor.is_some() {
            None
        } else {
            match state.not_found_mode {
                NotFoundMode::Root => route_component("__not_found_root__"),
                NotFoundMode::Fuzzy => state
                    .route_tree
                    .ancestors(&leaf_node_id)
                    .iter()
                    .rev()
                    .find_map(|ancestor| route_component(&format!("__not_found_{}", ancestor.id))),
            }
        };

        // The wrapper MUST fill the parent and act as a flex column.
        let mut wrapper = Node::div()
            .id(format!("outlet-{leaf_node_id}-{depth}"))
            .styles([
                quoin_ui::StyleCmd::SizeFull,
                quoin_ui::StyleCmd::Flex,
                quoin_ui::StyleCmd::FlexCol,
            ]);

        navi_core::context::provide(state.window_key, OutletDepth(depth + 1));

        if let Some(constructor) = constructor {
            wrapper = wrapper.child(constructor());
        } else if let Some(constructor) = not_found_constructor {
            wrapper = wrapper.child(constructor());
        } else {
            wrapper = wrapper.child("404 - Page not found");
        }

        wrapper = wrapper.children(self.children);
        wrapper.build()
    }
}
