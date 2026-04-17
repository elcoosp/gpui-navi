// navi-router/src/state.rs
use crate::blocker::{Blocker, BlockerId};
use crate::event_bus::push_event;
use crate::history::History;
use crate::loader::{CacheEntry, LoaderRegistry, LoaderState, LoaderTask};
use crate::location::{Location, NavigateOptions, ViewTransitionOptions};
use crate::route_tree::{RouteNode, RouteTree};
use gpui::{AnyWindowHandle, App, BorrowAppContext, EntityId, Global, WindowId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

/// Events emitted by the router during navigation lifecycle.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    fn name() -> &'static str;
}

/// The central router state.
pub struct RouterState {
    pub history: History,
    pub route_tree: Rc<RouteTree>,
    pub current_match: Option<(HashMap<String, String>, RouteNode)>,
    pub pending_navigation: Option<Location>,
    pub blockers: HashMap<BlockerId, Blocker>,
    pub scroll_restoration: bool,
    pub default_view_transition: Option<ViewTransitionOptions>,
    pub current_base: Option<String>,
    events: Vec<Box<dyn Fn(RouterEvent) + Send + Sync>>,
    next_blocker_id: BlockerId,
    loading: bool,
    loader_registry: LoaderRegistry,

    pub loader_cache: HashMap<String, CacheEntry>,
    pending_loaders: HashMap<String, LoaderTask>,
    pub loader_state: LoaderState,
    /// The window handle, used to refresh the UI after loader updates.
    window_handle: AnyWindowHandle,
    /// The EntityId of the root view, used to notify it after loader data changes.
    root_view: Option<EntityId>,
}

impl Global for RouterState {}

impl RouterState {
    pub fn new(
        initial: Location,
        window_id: WindowId,
        window_handle: AnyWindowHandle,
        route_tree: Rc<RouteTree>,
    ) -> Self {
        let current_match = route_tree
            .match_path(&initial.pathname)
            .map(|(params, node)| (params, node.clone()));
        Self {
            history: History::new(initial, window_id),
            route_tree,
            current_match,
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
            loader_state: LoaderState::Idle,
            window_handle,
            root_view: None,
        }
    }

    /// Set the root view entity ID. Called by the window after creating the root view.
    pub fn set_root_view(&mut self, view_id: EntityId) {
        self.root_view = Some(view_id);
    }

    /// Navigate to a new location.
    pub fn navigate(&mut self, loc: Location, options: NavigateOptions, cx: &mut App) {
        if !options.ignore_blocker {
            let current = self.current_location();
            for blocker in self.blockers.values() {
                if !blocker.should_allow(&current, &loc) {
                    self.pending_navigation = Some(loc);
                    return;
                }
            }
        }

        let from = Some(self.current_location());
        let to = loc.clone();

        // 1. BeforeNavigate
        push_event(
            RouterEvent::BeforeNavigate {
                from: from.clone(),
                to: to.clone(),
            },
            cx,
        );

        // 2. Update route match
        self.current_match = self
            .route_tree
            .match_path(&loc.pathname)
            .map(|(params, node)| (params, node.clone()));

        // 3. Update history
        if options.replace {
            self.history.replace(loc);
        } else {
            self.history.push(loc);
        }

        // 4. Resolved
        push_event(
            RouterEvent::Resolved {
                from: from.clone(),
                to: to.clone(),
            },
            cx,
        );

        // 5. Loader handling
        let has_loader = self
            .current_match
            .as_ref()
            .map(|(_, node)| node.has_loader)
            .unwrap_or(false);

        if has_loader {
            // Use the full method with explicit locations
            self.trigger_loader_with_locations(from, to, cx);
        } else {
            // No loader: proceed to mount and render
            push_event(
                RouterEvent::BeforeRouteMount {
                    from: from.clone(),
                    to: to.clone(),
                },
                cx,
            );
            if let Some(view_id) = self.root_view {
                cx.notify(view_id);
            }
            push_event(RouterEvent::Rendered { from, to }, cx);
        }
    }

    /// Preload a location without navigating (runs loaders in background).
    pub fn preload_location(&mut self, loc: Location, cx: &mut App) {
        // Find the route that would match this location
        if let Some((params, node)) = self.route_tree.match_path(&loc.pathname) {
            if node.has_loader {
                log::debug!("Preloading route: {}", node.id);
                let params_json = match serde_json::to_string(&params) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to serialize params for preload key: {}", e);
                        return;
                    }
                };
                let key = format!("{}:{}", node.id, params_json);

                // Only preload if not already cached or stale
                let should_load = if let Some(entry) = self.loader_cache.get(&key) {
                    let stale_time = node.loader_stale_time;
                    entry.is_stale(stale_time)
                } else {
                    true
                };

                if should_load && !self.pending_loaders.contains_key(&key) {
                    if let Some(loader_fn) = self.loader_registry.get(&node.id) {
                        let executor = cx.background_executor().clone();
                        let task = loader_fn(&params, executor, cx);
                        self.pending_loaders.insert(key.clone(), task);
                        self.loading = true;
                        self.loader_state = LoaderState::Loading {
                            route_id: node.id.clone(),
                        };

                        let key_clone = key.clone();
                        let node_id = node.id.clone();
                        let _stale_time = node.loader_stale_time;

                        cx.spawn(async move |cx| {
                            let task = cx.update_global::<RouterState, _>(|state, _| {
                                state.pending_loaders.remove(&key_clone)
                            });
                            if let Some(task) = task {
                                match task.await {
                                    Ok(data) => {
                                        let _ = cx.update_global::<RouterState, _>(|state, cx| {
                                            state.loader_cache.insert(
                                                key_clone,
                                                CacheEntry {
                                                    data,
                                                    inserted_at: Instant::now(),
                                                },
                                            );
                                            state.loading = state.pending_loaders.is_empty();
                                            state.loader_state = LoaderState::Idle;
                                            cx.refresh_windows();
                                        });
                                    }
                                    Err(e) => {
                                        let _ = cx.update_global::<RouterState, _>(|state, cx| {
                                            state.loading = state.pending_loaders.is_empty();
                                            state.loader_state = LoaderState::Error {
                                                route_id: node_id,
                                                message: e.to_string(),
                                            };
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
    pub fn proceed(&mut self, cx: &mut App) {
        if let Some(loc) = self.pending_navigation.take() {
            self.navigate(
                loc,
                NavigateOptions {
                    ignore_blocker: true,
                    ..Default::default()
                },
                cx,
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
        log::debug!("Registering loader for route: {}", route_id);
        self.loader_registry.insert(route_id, loader);
    }

    /// Legacy trigger_loader method (for backward compatibility).
    /// It determines the current navigation locations from the state.
    pub fn trigger_loader(&mut self, cx: &mut App) {
        let from = Some(self.current_location());
        let to = self
            .pending_navigation
            .clone()
            .unwrap_or_else(|| self.current_location());
        self.trigger_loader_with_locations(from, to, cx);
    }

    /// Trigger the loader for the current route with explicit navigation locations.
    /// `from` and `to` are the navigation locations that caused this loader to run.
    pub fn trigger_loader_with_locations(
        &mut self,
        from: Option<Location>,
        to: Location,
        cx: &mut App,
    ) {
        if let Some((params, node)) = &self.current_match {
            if node.has_loader {
                log::debug!("Loader trigger for route: {}", node.id);
                let params_json = match serde_json::to_string(params) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to serialize params for loader key: {}", e);
                        return;
                    }
                };
                let key = format!("{}:{}", node.id, params_json);
                log::debug!("Loader cache key: {}", key);

                // Check cache with staleness
                if let Some(entry) = self.loader_cache.get(&key) {
                    let stale_time = node.loader_stale_time;
                    if !entry.is_stale(stale_time) {
                        log::debug!("Loader cache hit (fresh) for key: {}", key);
                        push_event(
                            RouterEvent::Load {
                                from: from.clone(),
                                to: to.clone(),
                            },
                            cx,
                        );
                        push_event(
                            RouterEvent::BeforeRouteMount {
                                from: from.clone(),
                                to: to.clone(),
                            },
                            cx,
                        );
                        if let Some(view_id) = self.root_view {
                            cx.notify(view_id);
                        }
                        push_event(RouterEvent::Rendered { from, to }, cx);
                        return;
                    } else {
                        log::debug!("Loader cache hit but stale for key: {}, revalidating", key);
                        // Emit events immediately using stale data
                        push_event(
                            RouterEvent::Load {
                                from: from.clone(),
                                to: to.clone(),
                            },
                            cx,
                        );
                        push_event(
                            RouterEvent::BeforeRouteMount {
                                from: from.clone(),
                                to: to.clone(),
                            },
                            cx,
                        );
                        if let Some(view_id) = self.root_view {
                            cx.notify(view_id);
                        }
                        push_event(
                            RouterEvent::Rendered {
                                from: from.clone(),
                                to: to.clone(),
                            },
                            cx,
                        );
                        // Fall through to load fresh data in background
                    }
                }

                // Already pending – avoid duplicate
                if self.pending_loaders.contains_key(&key) {
                    log::debug!("Loader already pending for key: {}", key);
                    return;
                }

                // Emit BeforeLoad before starting async work
                push_event(
                    RouterEvent::BeforeLoad {
                        from: from.clone(),
                        to: to.clone(),
                    },
                    cx,
                );

                if let Some(loader_fn) = self.loader_registry.get(&node.id) {
                    log::debug!("Executing loader for route: {}", node.id);
                    let executor = cx.background_executor().clone();
                    let task = loader_fn(params, executor, cx);
                    self.pending_loaders.insert(key.clone(), task);
                    self.loading = true;
                    self.loader_state = LoaderState::Loading {
                        route_id: node.id.clone(),
                    };

                    let key_clone = key.clone();
                    let window_handle = self.window_handle;
                    let from_clone = from.clone();
                    let to_clone = to.clone();
                    let node_id = node.id.clone();
                    let _stale_time = node.loader_stale_time;
                    let root_view = self.root_view;

                    cx.spawn(async move |cx| {
                        let task = cx.update_global::<RouterState, _>(|state, _| {
                            state.pending_loaders.remove(&key_clone)
                        });

                        if let Some(task) = task {
                            match task.await {
                                Ok(data) => {
                                    log::debug!("Loader succeeded for key: {}", key_clone);
                                    let _ = cx.update_global::<RouterState, _>(|state, cx| {
                                        state.loader_cache.insert(
                                            key_clone,
                                            CacheEntry {
                                                data,
                                                inserted_at: Instant::now(),
                                            },
                                        );
                                        state.loading = state.pending_loaders.is_empty();
                                        state.loader_state = LoaderState::Idle;

                                        push_event(
                                            RouterEvent::Load {
                                                from: from_clone.clone(),
                                                to: to_clone.clone(),
                                            },
                                            cx,
                                        );
                                        push_event(
                                            RouterEvent::BeforeRouteMount {
                                                from: from_clone.clone(),
                                                to: to_clone.clone(),
                                            },
                                            cx,
                                        );
                                        if let Some(view_id) = root_view {
                                            cx.notify(view_id);
                                        }
                                        push_event(
                                            RouterEvent::Rendered {
                                                from: from_clone,
                                                to: to_clone,
                                            },
                                            cx,
                                        );
                                        cx.refresh_windows();
                                    });
                                    _ = window_handle.update(cx, |_, window, _| window.refresh());
                                }
                                Err(e) => {
                                    log::error!("Loader error for key {}: {}", key_clone, e);
                                    let _ = cx.update_global::<RouterState, _>(|state, cx| {
                                        state.loading = state.pending_loaders.is_empty();
                                        state.loader_state = LoaderState::Error {
                                            route_id: node_id,
                                            message: e.to_string(),
                                        };
                                        push_event(
                                            RouterEvent::Load {
                                                from: from_clone.clone(),
                                                to: to_clone.clone(),
                                            },
                                            cx,
                                        );
                                        push_event(
                                            RouterEvent::BeforeRouteMount {
                                                from: from_clone.clone(),
                                                to: to_clone.clone(),
                                            },
                                            cx,
                                        );
                                        if let Some(view_id) = root_view {
                                            cx.notify(view_id);
                                        }
                                        push_event(
                                            RouterEvent::Rendered {
                                                from: from_clone,
                                                to: to_clone,
                                            },
                                            cx,
                                        );
                                        cx.refresh_windows();
                                    });
                                    _ = window_handle.update(cx, |_, window, _| window.refresh());
                                }
                            }
                        }
                    })
                    .detach();
                } else {
                    log::warn!("No loader registered for route: {}", node.id);
                }
            }
        }
    }

    /// Get loader data for a specific route type.
    pub fn get_loader_data<R: crate::RouteDef>(&self) -> Option<R::LoaderData> {
        let (params, node) = self.current_match.as_ref()?;
        if node.id != R::name() {
            log::debug!(
                "Loader data request for {} but current route is {}",
                R::name(),
                node.id
            );
            return None;
        }
        let params_json = serde_json::to_string(params).ok()?;
        let key = format!("{}:{}", node.id, params_json);
        log::debug!("Looking up loader data with key: {}", key);
        let entry = self.loader_cache.get(&key)?;
        let arc_data = entry.data.downcast_ref::<Arc<R::LoaderData>>()?.clone();
        let data = (*arc_data).clone();
        log::debug!("Loader data found for key: {}", key);
        Some(data)
    }

    /// Returns the current loader state.
    pub fn loader_state(&self) -> &LoaderState {
        &self.loader_state
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
