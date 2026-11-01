use crate::{
    Blocker, BlockerId, Location, NavigateOptions, NotFound, Redirect, RouteNode, RouteTree,
    ViewTransitionOptions,
};
use gpui::{AnyWindowHandle, App, BorrowAppContext, EntityId, Global, WindowId};
use navi_router_core::{NavOp, NavigationEffect, RouterCore};
use rs_query::{Query, QueryClient, QueryKey};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub use navi_router_core::AnyData;

/// Path pushed when no route matches and no dedicated not-found route exists.
pub const NOT_FOUND_PATH: &str = "/404";

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

impl RouterEvent {
    /// The destination location of the event, when present.
    #[must_use]
    pub fn to(&self) -> Option<&Location> {
        match self {
            RouterEvent::BeforeNavigate { to, .. }
            | RouterEvent::BeforeLoad { to, .. }
            | RouterEvent::Load { to, .. }
            | RouterEvent::BeforeRouteMount { to, .. }
            | RouterEvent::Resolved { to, .. }
            | RouterEvent::Rendered { to, .. } => Some(to),
        }
    }
}

pub trait RouteDef: 'static {
    type Params: Clone + std::fmt::Debug + DeserializeOwned + 'static;
    type Search: Clone + std::fmt::Debug + 'static;
    type LoaderData: Clone + std::fmt::Debug + Send + Sync + 'static;
    fn path() -> &'static str;
    fn name() -> &'static str;
}

#[derive(Clone)]
pub enum LoaderOutcome<T> {
    Data(T),
    Redirect(Redirect),
    NotFound(NotFound),
}

impl<T: PartialEq> PartialEq for LoaderOutcome<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LoaderOutcome::Data(a), LoaderOutcome::Data(b)) => a == b,
            (LoaderOutcome::Redirect(a), LoaderOutcome::Redirect(b)) => a.to == b.to,
            (LoaderOutcome::NotFound(a), LoaderOutcome::NotFound(b)) => a.route_id == b.route_id,
            _ => false,
        }
    }
}

type LoaderFactory =
    Arc<dyn Fn(&HashMap<String, String>) -> Query<LoaderOutcome<AnyData>> + Send + Sync>;

#[derive(Clone, Debug, PartialEq)]
pub enum LoaderState {
    Idle,
    Loading,
    Ready,
    Error(String),
}

#[derive(Clone)]
pub enum NotFoundMode {
    Root,
    Fuzzy,
}

#[derive(Clone)]
pub struct RouterOptions {
    pub default_pending_ms: u64,
    pub default_pending_min_ms: u64,
    pub not_found_mode: NotFoundMode,
}

impl Default for RouterOptions {
    fn default() -> Self {
        Self {
            default_pending_ms: 1000,
            default_pending_min_ms: 500,
            not_found_mode: NotFoundMode::Root,
        }
    }
}

pub struct RouterState {
    pub(crate) core: RouterCore,
    pub route_tree: Rc<RouteTree>,
    pub current_match: Option<(HashMap<String, String>, Arc<RouteNode>)>,
    pub pending_navigation: Option<Location>,
    pub blockers: HashMap<BlockerId, Blocker>,
    pub scroll_restoration: bool,
    pub default_view_transition: Option<ViewTransitionOptions>,
    pub current_base: Option<String>,
    events: Vec<Box<dyn Fn(RouterEvent) + Send + Sync>>,
    next_blocker_id: BlockerId,
    pub query_client: QueryClient,
    loader_factories: HashMap<String, LoaderFactory>,
    context_cache: HashMap<String, serde_json::Value>,
    window_handle: AnyWindowHandle,
    root_view: Option<EntityId>,
    pub not_found_mode: NotFoundMode,
    pub not_found_data: Option<serde_json::Value>,
    pub default_pending_ms: u64,
    pub default_pending_min_ms: u64,
    /// Guards against infinite not-found recursion (see
    /// `handle_navigation_effects`).
    navigating_not_found: bool,
    /// Set when a navigation requested `reset_scroll`; the next
    /// [`ScrollRestoration`](crate::ScrollRestoration) paint consumes it and
    /// skips restoring the saved offset once.
    pub(crate) scroll_reset_pending: bool,
}

impl Global for RouterState {}

/// Build the canonical cache key for a route loader, shared by
/// [`RouterState::get_loader_data`] and [`RouterState::get_loader_state`].
fn loader_query_key<R: RouteDef>(
    route_id: &str,
    params: &HashMap<String, String>,
    node: &RouteNode,
    current_search: &serde_json::Value,
) -> QueryKey {
    let deps_json = node
        .loader_deps
        .as_ref()
        .map_or(serde_json::Value::Null, |f| f(current_search));
    let params_str = if std::any::TypeId::of::<R::Params>() == std::any::TypeId::of::<()>() {
        "null".to_string()
    } else {
        serde_json::to_string(params).unwrap_or_else(|_| "null".to_string())
    };
    let mut key_builder = QueryKey::new("navi_loader")
        .with("route", route_id)
        .with("params", params_str);
    if !deps_json.is_null() {
        key_builder = key_builder.with(
            "deps",
            serde_json::to_string(&deps_json).unwrap_or_default(),
        );
    }
    key_builder
}

/// Extract loader data from an [`AnyData`] produced by a route loader,
/// accepting both plain and `Arc`-wrapped payloads.
fn extract_loader_data<R: RouteDef>(any_data: &AnyData) -> Option<R::LoaderData> {
    if let Some(data) = any_data.downcast_ref::<R::LoaderData>() {
        return Some(data.clone());
    }
    any_data
        .downcast_ref::<Arc<R::LoaderData>>()
        .map(|arc| (**arc).clone())
}

impl RouterState {
    #[must_use]
    pub fn new(
        initial: Location,
        window_id: WindowId,
        window_handle: AnyWindowHandle,
        route_tree: Rc<RouteTree>,
    ) -> Self {
        Self::new_with_options(
            initial,
            window_id,
            window_handle,
            route_tree,
            RouterOptions::default(),
        )
    }

    #[must_use]
    pub fn new_with_options(
        initial: Location,
        _window_id: WindowId,
        window_handle: AnyWindowHandle,
        route_tree: Rc<RouteTree>,
        options: RouterOptions,
    ) -> Self {
        let core = RouterCore::new(initial, (*route_tree).clone());
        let current_match = core.current_match.clone();
        Self {
            core,
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
            context_cache: HashMap::new(),
            window_handle,
            root_view: None,
            not_found_mode: options.not_found_mode,
            not_found_data: None,
            default_pending_ms: options.default_pending_ms,
            default_pending_min_ms: options.default_pending_min_ms,
            navigating_not_found: false,
            scroll_reset_pending: false,
        }
    }

    // History delegates. The core re-matches on traversal; we mirror its
    // fresh `current_match` into the state copy the UI reads.
    pub fn back(&mut self) -> bool {
        let moved = self.core.back();
        if moved {
            self.current_match = self.core.current_match.clone();
        }
        moved
    }

    pub fn forward(&mut self) -> bool {
        let moved = self.core.forward();
        if moved {
            self.current_match = self.core.current_match.clone();
        }
        moved
    }

    pub fn go(&mut self, delta: isize) {
        self.core.go(delta);
        self.current_match = self.core.current_match.clone();
    }

    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.core.history().can_go_back()
    }

    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.core.history().can_go_forward()
    }

    pub fn set_root_view(&mut self, view_id: EntityId) {
        self.root_view = Some(view_id);
    }

    #[must_use]
    pub fn root_view(&self) -> Option<EntityId> {
        self.root_view
    }

    #[must_use]
    pub fn window_handle(&self) -> AnyWindowHandle {
        self.window_handle
    }

    /// Send an event to subscribers and to the global event log (devtools).
    fn dispatch(&self, event: &RouterEvent, cx: &mut App) {
        crate::event_bus::push_event(event.clone(), cx);
        self.emit(event);
    }

    pub fn navigate(&mut self, loc: Location, options: &NavigateOptions, cx: &mut App) {
        log::debug!("navigate called: {}", loc.pathname);

        self.dispatch(
            &RouterEvent::BeforeNavigate {
                from: Some(self.current_location()),
                to: loc.clone(),
            },
            cx,
        );

        // --- Blockers (async handling) ---
        if !options.ignore_blocker && !self.blockers.is_empty() {
            let current = self.current_location();
            let blockers: Vec<Blocker> = self.blockers.values().cloned().collect();
            let loc_clone = loc.clone();
            let options_clone = options.clone();
            cx.spawn({
                |cx: &mut gpui::AsyncApp| {
                    let cx = cx.clone();
                    async move {
                        let mut allow = true;
                        for blocker in &blockers {
                            if !blocker.should_allow(&current, &loc_clone).await {
                                allow = false;
                                break;
                            }
                        }
                        let () = cx.update(|cx| {
                            RouterState::update(cx, |state, cx| {
                                if allow {
                                    state.commit_navigation(loc_clone, &options_clone, cx);
                                } else {
                                    state.pending_navigation = Some(loc_clone);
                                }
                            });
                        });
                    }
                }
            })
            .detach();
            return;
        }

        self.commit_navigation(loc, options, cx);
    }

    fn commit_navigation(&mut self, loc: Location, options: &NavigateOptions, cx: &mut App) {
        let from = Some(self.current_location());

        if options.reset_scroll == Some(true) {
            self.scroll_reset_pending = true;
        }

        // Single match: the node is `Arc`-shared with the tree, no deep clone.
        let matched: Option<(HashMap<String, String>, Arc<RouteNode>)> =
            self.route_tree.match_path(&loc.pathname);
        let Some((_params, matched_node)) = matched.clone() else {
            // No match -> not found. Keep the core's effect payload in sync
            // with the state-level `not_found_data`.
            self.core.not_found_data = self.not_found_data.clone();
            let effects = self.core.navigate_with(loc.clone(), options, None);
            self.current_match = self.core.current_match.clone();
            self.handle_navigation_effects(effects, cx);
            self.dispatch(&RouterEvent::Resolved { from, to: loc }, cx);
            return;
        };

        let before_load_fns: Vec<crate::route_tree::BeforeLoadFn> = self
            .route_tree
            .ancestors(&matched_node.id)
            .iter()
            .filter_map(|node| node.before_load.clone())
            .collect();

        self.dispatch(
            &RouterEvent::BeforeLoad {
                from: from.clone(),
                to: loc.clone(),
            },
            cx,
        );

        if !before_load_fns.is_empty() {
            let window_handle = self.window_handle;
            let loc_clone = loc.clone();
            let options_clone = options.clone();

            cx.spawn(move |cx: &mut gpui::AsyncApp| {
                let cx = cx.clone();
                async move {
                    for before_load in before_load_fns {
                        let ctx = crate::route_tree::BeforeLoadContext {
                            params: HashMap::new(),
                            search: loc_clone.search.clone(),
                            location: loc_clone.clone(),
                        };
                        match before_load(ctx).await {
                            crate::route_tree::BeforeLoadResult::Ok => {}
                            crate::route_tree::BeforeLoadResult::Redirect(redirect) => {
                                let () = cx.update(|cx| {
                                    let nav = crate::Navigator::new(window_handle);
                                    nav.push_location(Location::new(&redirect.to), cx);
                                });
                                return;
                            }
                            crate::route_tree::BeforeLoadResult::NotFound(not_found) => {
                                let () = cx.update(|cx| {
                                    RouterState::update(cx, |state, cx| {
                                        state.not_found_data = not_found.data;
                                        if state.current_location().pathname != NOT_FOUND_PATH {
                                            let nav = crate::Navigator::new(state.window_handle());
                                            nav.push(NOT_FOUND_PATH, cx);
                                        }
                                    });
                                });
                                return;
                            }
                        }
                    }
                    // All before_load passed, proceed with core navigation
                    let () = cx.update(|cx| {
                        RouterState::update(cx, |state, cx| {
                            let effects = state.core.navigate_with(
                                loc_clone,
                                &options_clone,
                                matched.clone(),
                            );
                            state.current_match = state.core.current_match.clone();
                            state.handle_navigation_effects(effects, cx);
                        });
                    });
                }
            })
            .detach();
            return;
        }

        // No before_load hooks, direct navigation
        let effects = self.core.navigate_with(loc.clone(), options, matched);
        self.current_match = self.core.current_match.clone();
        self.handle_navigation_effects(effects, cx);
        self.dispatch(&RouterEvent::Resolved { from, to: loc }, cx);
    }

    fn handle_navigation_effects(&mut self, effects: Vec<NavigationEffect>, cx: &mut App) {
        for effect in effects {
            match effect {
                NavigationEffect::SpawnLoader { route_id, params } => {
                    if let Some(factory) = self.loader_factories.get(&route_id) {
                        let query = factory(&params);
                        let key = query.key.clone();
                        let client = self.query_client.clone();
                        let fetch_fn = query.fetch_fn.clone();
                        let query_options = query.options.clone();
                        let window_handle = self.window_handle;

                        cx.spawn(move |cx: &mut gpui::AsyncApp| {
                            let cx = cx.clone();
                            async move {
                                match (fetch_fn)().await {
                                    Ok(outcome) => match outcome {
                                        LoaderOutcome::Data(data) => {
                                            client.set_query_data(
                                                &key,
                                                data,
                                                query_options.clone(),
                                            );
                                            let () = cx.update(|cx| {
                                                RouterState::update(cx, |_, cx| {
                                                    cx.refresh_windows();
                                                });
                                                let _ = window_handle
                                                    .update(cx, |_, window, _| window.refresh());
                                            });
                                        }
                                        LoaderOutcome::Redirect(redirect) => {
                                            let () = cx.update(|cx| {
                                                let nav = crate::Navigator::new(window_handle);
                                                nav.push_location(Location::new(&redirect.to), cx);
                                            });
                                        }
                                        LoaderOutcome::NotFound(not_found) => {
                                            let () = cx.update(|cx| {
                                                RouterState::update(cx, |state, cx| {
                                                    state.not_found_data = not_found.data;
                                                    if state.current_location().pathname
                                                        != NOT_FOUND_PATH
                                                    {
                                                        let nav = crate::Navigator::new(
                                                            state.window_handle(),
                                                        );
                                                        nav.push(NOT_FOUND_PATH, cx);
                                                    }
                                                });
                                            });
                                        }
                                    },
                                    Err(e) => {
                                        log::error!("Loader error for {route_id}: {e:?}");
                                        let () = cx.update(|cx| {
                                            RouterState::update(cx, |_, cx| cx.refresh_windows());
                                            let _ = window_handle
                                                .update(cx, |_, window, _| window.refresh());
                                        });
                                    }
                                }
                            }
                        })
                        .detach();
                    }
                }
                NavigationEffect::Redirect { to, replace } => {
                    // Direct call: we are already inside `update_global`, so a
                    // nested `RouterState::update` (via `Navigator`) is illegal.
                    self.navigate(
                        Location::new(&to),
                        &NavigateOptions {
                            replace,
                            ..NavigateOptions::default()
                        },
                        cx,
                    );
                    return;
                }
                NavigationEffect::NotFound { data } => {
                    self.not_found_data = data;
                    // Direct call (no nested `update_global`) and a recursion
                    // guard for trees without a dedicated `/404` route: that
                    // navigation itself produces another NotFound effect.
                    if !self.navigating_not_found {
                        self.navigating_not_found = true;
                        self.navigate(
                            Location::new(NOT_FOUND_PATH),
                            &NavigateOptions::default(),
                            cx,
                        );
                        self.navigating_not_found = false;
                    }
                    return;
                }
                NavigationEffect::NotifyListeners => {
                    cx.refresh_windows();
                    let _ = self
                        .window_handle
                        .update(cx, |_, window, _| window.refresh());
                }
            }
        }
    }

    /// Apply a batch of navigation operations atomically (see
    /// [`RouterCore::transact`]): one event pair, one re-render, no
    /// interleaved async work. Location-changing ops still respect blockers:
    /// when any blocker is registered and the options do not ignore them,
    /// the batch falls back to sequential `navigate` calls so every hop goes
    /// through the blocker path.
    pub fn transact(&mut self, ops: &[NavOp], cx: &mut App) {
        let has_moves = ops
            .iter()
            .any(|op| matches!(op, NavOp::Push(_) | NavOp::Replace(_)));
        if has_moves && !self.blockers.is_empty() {
            for op in ops {
                match op {
                    NavOp::Push(loc) => {
                        self.navigate(loc.clone(), &NavigateOptions::default(), cx);
                    }
                    NavOp::Replace(loc) => {
                        self.navigate(
                            loc.clone(),
                            &NavigateOptions {
                                replace: true,
                                ..NavigateOptions::default()
                            },
                            cx,
                        );
                    }
                    NavOp::Back => {
                        self.back();
                    }
                    NavOp::Forward => {
                        self.forward();
                    }
                    NavOp::Go(delta) => {
                        self.go(*delta);
                    }
                }
            }
            return;
        }

        let from = Some(self.current_location());
        // The transaction resolves to the last location-changing op.
        let to = ops.iter().rev().find_map(|op| match op {
            NavOp::Push(loc) | NavOp::Replace(loc) => Some(loc.clone()),
            _ => None,
        });
        if let Some(to) = &to {
            self.dispatch(
                &RouterEvent::BeforeNavigate {
                    from: from.clone(),
                    to: to.clone(),
                },
                cx,
            );
        }
        let effects = self.core.transact(ops);
        self.current_match = self.core.current_match.clone();
        self.handle_navigation_effects(effects, cx);
        if let Some(to) = to {
            self.dispatch(&RouterEvent::Resolved { from, to }, cx);
        }
    }

    /// Consume the one-shot flag set by a navigation with
    /// `reset_scroll: Some(true)`. [`ScrollRestoration`](crate::ScrollRestoration)
    /// uses it to skip restoring the saved offset exactly once.
    pub fn take_scroll_reset(&mut self) -> bool {
        std::mem::take(&mut self.scroll_reset_pending)
    }

    pub fn preload_location(&mut self, loc: &Location, cx: &mut App) {
        if let Some((params, node)) = self.route_tree.match_path(&loc.pathname)
            && node.has_loader
            && let Some(factory) = self.loader_factories.get(&node.id)
        {
            let stale_time = node.loader_stale_time.unwrap_or(std::time::Duration::ZERO);
            let gc_time = node
                .loader_gc_time
                .unwrap_or(std::time::Duration::from_secs(300));
            let query = factory(&params).stale_time(stale_time).gc_time(gc_time);
            let key = query.key.clone();
            let client = self.query_client.clone();
            let fetch_fn = query.fetch_fn.clone();
            let options = query.options.clone();
            let node_id = node.id.clone();
            cx.spawn(|_cx: &mut gpui::AsyncApp| async move {
                match (fetch_fn)().await {
                    Ok(LoaderOutcome::Data(data)) => {
                        client.set_query_data(&key, data, options);
                    }
                    Ok(LoaderOutcome::Redirect(_) | LoaderOutcome::NotFound(_)) => {}
                    Err(e) => {
                        log::error!("Preload error for {node_id}: {e}");
                    }
                }
            })
            .detach();
        }
    }

    #[must_use]
    pub fn current_location(&self) -> Location {
        self.core.current_location()
    }

    pub fn emit(&self, event: &RouterEvent) {
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
                &NavigateOptions {
                    ignore_blocker: true,
                    ..NavigateOptions::default()
                },
                cx,
            );
        }
    }

    pub fn reset_block(&mut self) {
        self.pending_navigation = None;
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.pending_navigation.is_some()
    }

    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.query_client.is_fetching()
    }

    pub fn register_loader_factory(&mut self, route_id: &str, factory: LoaderFactory) {
        self.loader_factories.insert(route_id.to_string(), factory);
    }

    fn loader_key_for_current<R: RouteDef>(&self) -> Option<QueryKey> {
        let (params, node) = self.current_match.as_ref()?;
        if node.id != R::name() {
            return None;
        }
        Some(loader_query_key::<R>(
            &node.id,
            params,
            node,
            &self.current_location().search,
        ))
    }

    #[must_use]
    pub fn get_loader_data<R: RouteDef>(&self) -> Option<R::LoaderData> {
        let key = self.loader_key_for_current::<R>()?;
        let any_data: AnyData = self.query_client.get_query_data(&key)?;
        extract_loader_data::<R>(&any_data)
    }

    #[must_use]
    pub fn get_loader_state<R: RouteDef>(&self) -> LoaderState {
        let Some(key) = self.loader_key_for_current::<R>() else {
            return LoaderState::Idle;
        };
        if self.query_client.is_in_flight(&key) {
            LoaderState::Loading
        } else if self.query_client.get_query_data::<AnyData>(&key).is_some() {
            LoaderState::Ready
        } else {
            LoaderState::Idle
        }
    }

    #[must_use]
    pub fn get_route_context<R: RouteDef>(&self) -> Option<serde_json::Value> {
        let node = &self.current_match.as_ref()?.1;
        if node.id != R::name() {
            return None;
        }
        self.context_cache.get(&node.id).cloned()
    }

    pub fn set_route_context<R: RouteDef>(&mut self, value: serde_json::Value) {
        self.context_cache.insert(R::name().to_string(), value);
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

    #[must_use]
    pub fn current_meta(&self) -> HashMap<String, serde_json::Value> {
        let mut meta = HashMap::new();
        if let Some((_, node)) = &self.current_match {
            for ancestor in self.route_tree.ancestors(&node.id) {
                meta.extend(ancestor.meta.clone());
            }
            meta.extend(node.meta.clone());
        }
        meta
    }

    #[must_use]
    pub fn has_pending_loader(&self) -> bool {
        self.query_client.is_fetching()
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }
}
