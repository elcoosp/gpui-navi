use crate::blocker::{Blocker, BlockerId};
use crate::event_bus::push_event;
use crate::history::History;
use crate::loader::LoaderRegistry;
use crate::loader_query::create_loader_query;
use crate::location::{Location, NavigateOptions, ViewTransitionOptions};
use crate::route_tree::{RouteNode, RouteTree};
use gpui::{AnyWindowHandle, App, BorrowAppContext, EntityId, Global, WindowId};
use rs_query::{QueryClient, QueryKey, QueryOptions};
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

    // Replaced old cache with rs-query client
    pub query_client: QueryClient,

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
            query_client: QueryClient::new(),
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
                let query_key = QueryKey::new(&key);

                // Check rs-query cache and staleness
                let should_load =
                    if let Some(entry) = self.query_client.cache.get(query_key.cache_key()) {
                        let stale_time = node.loader_stale_time.unwrap_or(Duration::ZERO);
                        entry.fetched_at.elapsed() > stale_time
                    } else {
                        true
                    };

                if should_load && !self.query_client.is_in_flight(&query_key) {
                    if let Some(loader_fn) = self.loader_registry.get(&node.id) {
                        let executor = cx.background_executor().clone();
                        let params_clone = params.clone();
                        let node_clone = node.clone();
                        let client = self.query_client.clone();

                        // Build rs-query query
                        let query: rs_query::Query<Arc<dyn std::any::Any + Send + Sync>> = {
                            let key = query_key.clone();
                            let stale_time = node_clone.loader_stale_time.unwrap_or(Duration::ZERO);
                            let gc_time = node_clone
                                .loader_gc_time
                                .unwrap_or(Duration::from_secs(300));
                            let options = QueryOptions {
                                stale_time,
                                gc_time,
                                ..Default::default()
                            };
                            let fetch_fn = move || {
                                let loader = loader_fn.clone();
                                let params = params_clone.clone();
                                let exec = executor.clone();
                                async move { loader(&params, exec, &mut App::default()).await }
                            };
                            rs_query::Query::new(key, fetch_fn).options(options)
                        };

                        // Spawn the query in background (no UI update needed for preload)
                        let _ = cx.foreground_executor().spawn(async move {
                            let _ = rs_query::execute_query(&client, &query).await;
                        });
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
        // rs-query tracks its own fetching state; we can also check manually
        self.loading || self.query_client.is_fetching()
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
                let query_key = QueryKey::new(&key);
                log::debug!("Loader cache key: {}", key);

                // Check rs-query cache with staleness
                let stale_time = node.loader_stale_time.unwrap_or(Duration::ZERO);
                let is_fresh =
                    if let Some(entry) = self.query_client.cache.get(query_key.cache_key()) {
                        entry.fetched_at.elapsed() <= stale_time
                    } else {
                        false
                    };

                if is_fresh {
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
                } else if self.query_client.cache.contains_key(query_key.cache_key()) {
                    // Stale cache exists: render immediately with stale data, then refetch in background
                    log::debug!("Loader cache hit but stale for key: {}, revalidating", key);
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
                    // Continue to background fetch
                }

                // Already pending – avoid duplicate
                if self.query_client.is_in_flight(&query_key) {
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

                    // Use the helper to create the rs-query Query
                    let query =
                        create_loader_query(&node.id, &params_json, node, loader_fn.clone(), cx);

                    let client = self.query_client.clone();
                    let window_handle = self.window_handle;
                    let from_clone = from.clone();
                    let to_clone = to.clone();
                    let root_view = self.root_view;

                    self.loading = true;

                    cx.spawn(async move |cx| {
                        // Execute the query via rs-query
                        let _state = rs_query::execute_query(&client, &query).await;
                        let _ = cx.update_global::<RouterState, _>(|state, cx| {
                            state.loading = false;
                            // Notify UI after query completes (success or error)
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
                        let _ = window_handle.update(cx, |_, window, _| window.refresh());
                    })
                    .detach();
                } else {
                    log::warn!("No loader registered for route: {}", node.id);
                }
            }
        }
    }

    /// Get loader data for a specific route type using rs-query cache.
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
        let query_key = QueryKey::new(&key);
        log::debug!("Looking up loader data with key: {}", key);
        let arc_any: Option<Arc<dyn std::any::Any + Send + Sync>> =
            self.query_client.get_query_data(&query_key);
        arc_any
            .and_then(|arc| arc.downcast_ref::<Arc<R::LoaderData>>().cloned())
            .map(|arc| (*arc).clone())
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
