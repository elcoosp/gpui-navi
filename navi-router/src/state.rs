use crate::blocker::{Blocker, BlockerId};
use crate::history::History;
use crate::loader::{LoaderRegistry, LoaderTask};
use crate::location::{Location, NavigateOptions, ViewTransitionOptions};
use crate::route_tree::{RouteNode, RouteTree};
use gpui::{App, BorrowAppContext, Global, WindowId};
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
    loader_registry: LoaderRegistry,
    loader_cache: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
    pending_loaders: HashMap<String, LoaderTask>,
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
            loader_registry: LoaderRegistry::new(),
            loader_cache: HashMap::new(),
            pending_loaders: HashMap::new(),
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
        self.loading || !self.pending_loaders.is_empty()
    }

    /// Register a loader function for a route.
    pub fn register_loader(&mut self, route_id: &str, loader: crate::loader::LoaderFn) {
        self.loader_registry.insert(route_id, loader);
    }

    /// Trigger the loader for the current route.
    pub fn trigger_loader(&mut self, cx: &mut App) {
        if let Some((params, node)) = &self.current_match {
            if node.has_loader {
                let key = format!(
                    "{}:{}",
                    node.id,
                    serde_json::to_string(params).unwrap_or_default()
                );
                if self.loader_cache.contains_key(&key) || self.pending_loaders.contains_key(&key) {
                    return;
                }
                if let Some(loader_fn) = self.loader_registry.get(&node.id) {
                    let executor = cx.background_executor().clone();
                    let task = loader_fn(params, executor, cx);
                    self.pending_loaders.insert(key.clone(), task);
                    self.loading = true;

                    let key_clone = key.clone();
                    cx.spawn(async move |cx| {
                        let task = cx.update_global::<RouterState, _>(|state, _| {
                            state.pending_loaders.remove(&key_clone)
                        });

                        if let Ok(Some(task)) = task {
                            match task.await {
                                Ok(data) => {
                                    let _ = cx.update_global::<RouterState, _>(|state, cx| {
                                        state.loader_cache.insert(key_clone, data);
                                        state.loading = state.pending_loaders.is_empty();
                                        cx.refresh_windows();
                                    });
                                }
                                Err(e) => {
                                    eprintln!("Loader error: {}", e);
                                    let _ = cx.update_global::<RouterState, _>(|state, cx| {
                                        state.loading = state.pending_loaders.is_empty();
                                        cx.refresh_windows();
                                    });
                                }
                            }
                        }
                    })
                    .detach();
                }
            }
        }
    }

    /// Get loader data for a specific route type.
    pub fn get_loader_data<R: crate::RouteDef>(&self) -> Option<R::LoaderData> {
        let (params, node) = self.current_match.as_ref()?;
        if node.id != R::path() {
            return None;
        }
        let key = format!(
            "{}:{}",
            node.id,
            serde_json::to_string(params).unwrap_or_default()
        );
        self.loader_cache
            .get(&key)?
            .downcast_ref::<R::LoaderData>()
            .cloned()
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
