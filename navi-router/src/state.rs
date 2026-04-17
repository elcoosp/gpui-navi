// navi-router/src/state.rs
use crate::blocker::{Blocker, BlockerId};
use crate::event_bus::push_event;
use crate::history::History;
use crate::location::{Location, NavigateOptions, ViewTransitionOptions};
use crate::route_tree::{RouteNode, RouteTree};
use gpui::{AnyWindowHandle, App, BorrowAppContext, EntityId, Global, WindowId};
use rs_query::{Query, QueryClient, QueryKey};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

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

// Wrapper that implements PartialEq for any type.
#[derive(Clone)]
pub struct AnyData(pub Arc<dyn std::any::Any + Send + Sync>);

impl PartialEq for AnyData {
    fn eq(&self, _other: &Self) -> bool {
        false // Always treat as not equal to force cache updates
    }
}

impl std::fmt::Debug for AnyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyData").finish()
    }
}

type LoaderFactory = Arc<dyn Fn(&HashMap<String, String>) -> Query<AnyData> + Send + Sync>;

/// Loader state for a route.
#[derive(Clone, Debug, PartialEq)]
pub enum LoaderState {
    Idle,
    Loading,
    Ready,
    Error(String),
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

    // --- rs-query integration ---
    pub query_client: QueryClient,
    loader_factories: HashMap<String, LoaderFactory>,

    /// The window handle, used to refresh the UI after loader updates.
    window_handle: AnyWindowHandle,
    /// The EntityId of the root view, kept for future use (e.g., view transitions).
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
            query_client: QueryClient::new(),
            loader_factories: HashMap::new(),
            window_handle,
            root_view: None,
        }
    }

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

        push_event(
            RouterEvent::BeforeNavigate {
                from: from.clone(),
                to: to.clone(),
            },
            cx,
        );

        self.current_match = self
            .route_tree
            .match_path(&loc.pathname)
            .map(|(params, node)| (params, node.clone()));

        if options.replace {
            self.history.replace(loc);
        } else {
            self.history.push(loc);
        }

        push_event(
            RouterEvent::Resolved {
                from: from.clone(),
                to: to.clone(),
            },
            cx,
        );

        self.trigger_loader_with_locations(from, to, cx);
    }

    /// Preload a location without navigating (runs loaders in background).
    pub fn preload_location(&mut self, loc: Location, cx: &mut App) {
        if let Some((params, node)) = self.route_tree.match_path(&loc.pathname) {
            if node.has_loader {
                if let Some(factory) = self.loader_factories.get(&node.id) {
                    let query = factory(&params);
                    let key = query.key.clone();
                    let client = self.query_client.clone();
                    let fetch_fn = query.fetch_fn.clone();
                    let options = query.options.clone();
                    let node_id = node.id.clone();
                    cx.spawn(|_cx: &mut gpui::AsyncApp| {
                        let _cx = _cx.clone(); // Clone to own the AsyncApp
                        async move {
                            match (fetch_fn)().await {
                                Ok(data) => {
                                    client.set_query_data(&key, data, options);
                                }
                                Err(e) => {
                                    log::error!("Preload error for {}: {}", node_id, e);
                                }
                            }
                        }
                    })
                    .detach();
                }
            }
        }
    }

    pub fn current_location(&self) -> Location {
        self.history.current()
    }

    pub fn emit(&self, event: RouterEvent) {
        for listener in &self.events {
            listener(event.clone());
        }
    }

    pub fn subscribe<F: Fn(RouterEvent) + Send + Sync + 'static>(&mut self, f: F) {
        self.events.push(Box::new(f));
    }

    pub fn add_blocker(&mut self, blocker: Blocker) -> BlockerId {
        let id = self.next_blocker_id;
        self.next_blocker_id += 1;
        self.blockers.insert(id, blocker);
        id
    }

    pub fn remove_blocker(&mut self, id: &BlockerId) {
        self.blockers.remove(id);
    }

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

    pub fn reset_block(&mut self) {
        self.pending_navigation = None;
    }

    pub fn is_blocked(&self) -> bool {
        self.pending_navigation.is_some()
    }

    pub fn is_loading(&self) -> bool {
        self.query_client.is_fetching()
    }

    /// Register a loader factory for a route.
    pub fn register_loader_factory(&mut self, route_id: &str, factory: LoaderFactory) {
        log::debug!(
            "Registering rs-query loader factory for route: {}",
            route_id
        );
        self.loader_factories.insert(route_id.to_string(), factory);
    }

    fn trigger_loader_with_locations(
        &mut self,
        from: Option<Location>,
        to: Location,
        cx: &mut App,
    ) {
        if let Some((params, node)) = &self.current_match {
            if node.has_loader {
                let route_id = node.id.clone();
                if let Some(factory) = self.loader_factories.get(&route_id) {
                    let query = factory(params);
                    let key = query.key.clone();
                    let client = self.query_client.clone();
                    let fetch_fn = query.fetch_fn.clone();
                    let options = query.options.clone();

                    push_event(
                        RouterEvent::BeforeLoad {
                            from: from.clone(),
                            to: to.clone(),
                        },
                        cx,
                    );

                    let window_handle = self.window_handle;
                    let from_clone = from.clone();
                    let to_clone = to.clone();
                    cx.spawn(move |cx: &mut gpui::AsyncApp| {
                        let cx = cx.clone(); // Clone to own the AsyncApp
                        async move {
                            match (fetch_fn)().await {
                                Ok(data) => {
                                    client.set_query_data(&key, data, options);
                                    let _ = cx.update(|cx| {
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
                                        push_event(
                                            RouterEvent::Rendered {
                                                from: from_clone,
                                                to: to_clone,
                                            },
                                            cx,
                                        );
                                        let _ = window_handle
                                            .update(cx, |_, window, _| window.refresh());
                                    });
                                }
                                Err(e) => {
                                    log::error!("Loader error for {}: {:?}", route_id, e);
                                    let _ = cx.update(|cx| {
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
                                        push_event(
                                            RouterEvent::Rendered {
                                                from: from_clone,
                                                to: to_clone,
                                            },
                                            cx,
                                        );
                                        let _ = window_handle
                                            .update(cx, |_, window, _| window.refresh());
                                    });
                                }
                            }
                        }
                    })
                    .detach();
                } else {
                    log::warn!("No loader factory registered for route: {}", route_id);
                    self.proceed_without_loader(from, to, cx);
                }
            } else {
                self.proceed_without_loader(from, to, cx);
            }
        }
    }

    fn proceed_without_loader(&self, from: Option<Location>, to: Location, cx: &mut App) {
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
        push_event(RouterEvent::Rendered { from, to }, cx);
    }

    /// Get loader data for a specific route type.
    pub fn get_loader_data<R: crate::RouteDef>(&self) -> Option<R::LoaderData> {
        let (params, node) = self.current_match.as_ref()?;
        if node.id != R::name() {
            return None;
        }
        let key = QueryKey::new("navi_loader")
            .with("route", node.id.as_str())
            .with("params", serde_json::to_string(params).ok()?);

        let any_data: AnyData = self.query_client.get_query_data(&key)?;
        let arc_data = any_data.0.downcast_ref::<Arc<R::LoaderData>>()?.clone();
        Some((*arc_data).clone())
    }

    /// Get the loader state for a specific route type.
    pub fn get_loader_state<R: crate::RouteDef>(&self) -> LoaderState {
        let (params, node) = match &self.current_match {
            Some(m) => m,
            None => return LoaderState::Idle,
        };
        if node.id != R::name() {
            return LoaderState::Idle;
        }
        let key = QueryKey::new("navi_loader")
            .with("route", node.id.as_str())
            .with("params", serde_json::to_string(params).unwrap_or_default());

        if self.query_client.is_in_flight(&key) {
            LoaderState::Loading
        } else if self.query_client.get_query_data::<AnyData>(&key).is_some() {
            LoaderState::Ready
        } else {
            LoaderState::Idle
        }
    }

    /// Check if any loader is currently pending.
    pub fn has_pending_loader(&self) -> bool {
        self.query_client.is_fetching()
    }

    // Global access helpers
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
