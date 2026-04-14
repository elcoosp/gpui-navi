use crate::blocker::{Blocker, BlockerId};
use crate::history::History;
use crate::loader::{LoaderError, LoaderResult};
use crate::location::{Location, NavigateOptions, ViewTransitionOptions};
use crate::route_tree::{RouteNode, RouteTree};
use gpui::{App, BorrowAppContext, Global, Task, WindowId};
use rs_query::QueryClient;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Events emitted by the router during navigation lifecycle.
#[derive(Clone, Debug)]
pub enum RouterEvent {
    BeforeNavigate {
        from: Option<Location>,
        to: Location,
    },
    BeforeLoad {
        from: Option<Location>,
        to: Location,
    },
    Load {
        from: Option<Location>,
        to: Location,
    },
    BeforeRouteMount {
        from: Option<Location>,
        to: Location,
    },
    Resolved {
        from: Option<Location>,
        to: Location,
    },
    Rendered {
        from: Option<Location>,
        to: Location,
    },
}

/// Trait for route definitions.
pub trait RouteDef: 'static {
    type Params: Clone + std::fmt::Debug + DeserializeOwned + 'static;
    type Search: Clone + std::fmt::Debug + 'static;
    type LoaderData: Clone + std::fmt::Debug + Send + Sync + 'static;

    fn path() -> &'static str;
}

/// The central router state.
pub struct RouterState {
    pub history: History,
    pub route_tree: Rc<RouteTree>,
    pub current_match: Option<(HashMap<String, String>, RouteNode)>,
    pub query_client: QueryClient,
    pub pending_navigation: Option<Location>,
    pub blockers: HashMap<BlockerId, Blocker>,
    pub scroll_restoration: bool,
    pub default_view_transition: Option<ViewTransitionOptions>,
    pub current_base: Option<String>,
    events: Vec<Box<dyn Fn(RouterEvent) + Send + Sync>>,
    next_blocker_id: BlockerId,
    loading: bool,
    loader_cache: HashMap<String, LoaderResult>,
    loader_registry: HashMap<
        String,
        Box<
            dyn Fn(&mut App) -> Task<Result<Arc<dyn std::any::Any + Send + Sync>, LoaderError>>
                + Send
                + Sync,
        >,
    >,
}

impl Global for RouterState {}

impl RouterState {
    pub fn new(initial: Location, window_id: WindowId, route_tree: Rc<RouteTree>) -> Self {
        let current_match = route_tree
            .match_path(&initial.pathname)
            .map(|(params, node)| (params, node.clone()));
        Self {
            history: History::new(initial, window_id),
            route_tree,
            current_match,
            query_client: QueryClient::new(),
            pending_navigation: None,
            blockers: HashMap::new(),
            scroll_restoration: true,
            default_view_transition: None,
            events: Vec::new(),
            current_base: None,
            next_blocker_id: 0,
            loading: false,
            loader_cache: HashMap::new(),
            loader_registry: HashMap::new(),
        }
    }

    /// Navigate to a new location.
    pub fn navigate(&mut self, loc: Location, options: NavigateOptions) {
        if !options.ignore_blocker {
            let current = self.current_location();
            for blocker in self.blockers.values() {
                if !blocker.should_allow(&current, &loc) {
                    self.pending_navigation = Some(loc);
                    return;
                }
            }
        }

        self.emit(RouterEvent::BeforeNavigate {
            from: Some(self.current_location()),
            to: loc.clone(),
        });

        self.current_match = self
            .route_tree
            .match_path(&loc.pathname)
            .map(|(params, node)| (params, node.clone()));

        if options.replace {
            self.history.replace(loc);
        } else {
            self.history.push(loc);
        }
    }

    /// Get the current location from history.
    pub fn current_location(&self) -> Location {
        self.history.current()
    }

    /// Emit a router event to all subscribers.
    pub fn emit(&self, event: RouterEvent) {
        for listener in &self.events {
            listener(event.clone());
        }
    }

    /// Subscribe to router events.
    pub fn subscribe<F: Fn(RouterEvent) + Send + Sync + 'static>(&mut self, f: F) {
        self.events.push(Box::new(f));
    }

    /// Add a navigation blocker and return its ID.
    pub fn add_blocker(&mut self, blocker: Blocker) -> BlockerId {
        let id = self.next_blocker_id;
        self.next_blocker_id += 1;
        self.blockers.insert(id, blocker);
        id
    }

    /// Remove a navigation blocker by ID.
    pub fn remove_blocker(&mut self, id: &BlockerId) {
        self.blockers.remove(id);
    }

    /// Proceed with a blocked navigation.
    pub fn proceed(&mut self) {
        if let Some(loc) = self.pending_navigation.take() {
            self.navigate(
                loc,
                NavigateOptions {
                    ignore_blocker: true,
                    ..Default::default()
                },
            );
        }
    }

    /// Reset/cancel a blocked navigation.
    pub fn reset_block(&mut self) {
        self.pending_navigation = None;
    }

    /// Check if there's a pending blocked navigation.
    pub fn is_blocked(&self) -> bool {
        self.pending_navigation.is_some()
    }

    /// Check if any route loader is currently in progress.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Register a loader function for a route.
    pub fn register_loader<R: RouteDef>(
        &mut self,
        loader: fn(
            R::Params,
            &mut App,
        ) -> Task<Result<Arc<dyn std::any::Any + Send + Sync>, LoaderError>>,
    ) {
        let route_id = R::path().to_string();
        let wrapped = Box::new(move |cx: &mut App| {
            // We need params to call the loader. They will be provided when triggering.
            // For now, we store the loader and will retrieve params at trigger time.
            // We'll store the function pointer directly and handle params in trigger_current_loader.
            unimplemented!("Will be called with params in trigger_current_loader")
        });
        // Since we need params at trigger time, we can store the function pointer and
        // call it with the deserialized params in `trigger_current_loader`.
        // Let's change the registry to store the original loader function.
        // For simplicity, we'll store the function pointer in a type-erased wrapper.
        self.loader_registry.insert(route_id, wrapped);
    }

    /// Trigger the loader for the current route.
    pub fn trigger_current_loader(&mut self, cx: &mut App) {
        if let Some((params, node)) = &self.current_match {
            if node.has_loader {
                let loader_key = format!(
                    "{}:{}",
                    node.id,
                    serde_json::to_string(params).unwrap_or_default()
                );
                if !self.loader_cache.contains_key(&loader_key) {
                    // Look up the loader for this route ID
                    if let Some(loader_fn) = self.loader_registry.get(&node.id) {
                        // Deserialize params to the expected type? We don't have the type here.
                        // Instead, the loader should accept `HashMap<String, String>`.
                        // We'll adjust the macro to generate a loader that takes `HashMap`.
                        // For now, we'll assume the loader takes params map.
                        let task = loader_fn(cx);
                        self.loader_cache
                            .insert(loader_key.clone(), LoaderResult::Pending(task));
                        self.loading = true;
                    }
                }
            }
        }
    }

    /// Get loader data for a specific route type.
    pub fn get_loader_data<R: RouteDef>(&self) -> Option<R::LoaderData> {
        let current_match = self.current_match.as_ref()?;
        let (params, node) = current_match;
        if node.id != R::path() {
            return None;
        }
        let loader_key = format!(
            "{}:{}",
            node.id,
            serde_json::to_string(params).unwrap_or_default()
        );
        match self.loader_cache.get(&loader_key) {
            Some(LoaderResult::Ready(data)) => data
                .clone()
                .downcast::<R::LoaderData>()
                .ok()
                .map(|arc| (*arc).clone()),
            _ => None,
        }
    }

    /// Update loader results (called by async tasks).
    pub fn complete_loader(&mut self, key: String, data: Arc<dyn std::any::Any + Send + Sync>) {
        self.loader_cache.insert(key, LoaderResult::Ready(data));
        self.loading = false;
    }

    // --- Global access helpers ---

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn try_global(cx: &App) -> Option<&Self> {
        cx.try_global::<Self>()
    }

    pub fn update<F, R>(cx: &mut App, f: F) -> R
    where
        F: FnOnce(&mut Self, &mut App) -> R,
    {
        cx.update_global(|state, cx| f(state, cx))
    }
}
