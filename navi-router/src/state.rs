use crate::blocker::{Blocker, BlockerId};
use crate::event_bus::push_event;
use crate::history::History;
use crate::loader::{LoaderRegistry, LoaderTask};
use crate::location::{Location, NavigateOptions, ViewTransitionOptions};
use crate::route_tree::{RouteNode, RouteTree};
use gpui::{AnyWindowHandle, App, BorrowAppContext, EntityId, Global, WindowId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

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

    loader_cache: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
    pending_loaders: HashMap<String, LoaderTask>,
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

                // Cache hit – data already available
                if self.loader_cache.contains_key(&key) {
                    log::debug!("Loader cache hit for key: {}", key);
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

                    let key_clone = key.clone();
                    let window_handle = self.window_handle;
                    let from_clone = from.clone();
                    let to_clone = to.clone();

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
                                            data.clone() as Arc<dyn std::any::Any + Send + Sync>,
                                        );
                                        state.loading = state.pending_loaders.is_empty();

                                        // Emit Load, BeforeRouteMount, Rendered
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
                                        if let Some(view_id) = state.root_view {
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
                                    cx.background_executor()
                                        .timer(Duration::from_millis(16))
                                        .await;
                                    _ = window_handle.update(cx, |_, window, _| window.refresh());
                                }
                                Err(e) => {
                                    log::error!("Loader error for key {}: {}", key_clone, e);
                                    let _ = cx.update_global::<RouterState, _>(|state, cx| {
                                        state.loading = state.pending_loaders.is_empty();
                                        // Even on error, emit Load and let the route render
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
                                        if let Some(view_id) = state.root_view {
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
        let arc_data = self
            .loader_cache
            .get(&key)?
            .downcast_ref::<Arc<R::LoaderData>>()?
            .clone();
        let data = (*arc_data).clone();
        log::debug!("Loader data found for key: {}", key);
        Some(data)
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
