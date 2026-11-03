//! Router state, navigation host seam, and loader wiring.
//!
//! The whole crate is GUI-framework agnostic: state mutations flow through
//! a shared [`RouterHandle`], async work runs on the quoin-backed
//! [`RouterHost`] seam (UI-thread spawn + repaint request), and rendering
//! happens through `quoin_ui::Node` trees. The embedding application binds
//! the host to its framework of choice.

use crate::{
    Blocker, BlockerId, Location, NavigateOptions, NotFound, Redirect, RouteNode, RouteTree,
    ViewTransitionOptions,
};
use navi_core::WindowKey;
use navi_router_core::{NavOp, NavigationEffect, RouterCore};
use parking_lot::{Mutex, RwLock};
use rs_query::{Query, QueryClient, QueryKey};
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cell::OnceCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

pub use navi_router_core::AnyData;

/// Path pushed when no route matches and no dedicated not-found route exists.
pub const NOT_FOUND_PATH: &str = "/404";

/// A boxed, UI-thread future — the currency of [`RouterHost::spawn`].
pub type LocalFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

/// The seam between the router and the embedding runtime.
///
/// * `spawn` runs a future on the UI thread (parity with the classic
///   foreground `cx.spawn` flows: loader completion, blocker resolution,
///   before-load guards).
/// * `refresh` requests a repaint of the hosting window(s) — the quoin
///   `request_update` contract; the application maps it onto its framework
///   (e.g. a window-wide refresh notifier).
///
/// Both hooks are UI-thread-only (`Rc`), matching every mainstream
/// single-threaded UI runtime; cross-framework adapters queue onto their
/// main thread instead of requiring `Send` futures.
#[derive(Clone)]
pub struct RouterHost {
    /// Requests a UI repaint. Callable from the UI thread.
    pub refresh: Rc<dyn Fn()>,
    /// Spawns a UI-thread future.
    pub spawn: Rc<dyn Fn(LocalFuture)>,
}

impl RouterHost {
    /// Spawns a UI-thread future through the seam.
    pub fn spawn(&self, future: LocalFuture) {
        (self.spawn)(future);
    }

    /// Requests a repaint through the seam.
    pub fn refresh(&self) {
        (self.refresh)();
    }
}

impl std::fmt::Debug for RouterHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterHost").finish_non_exhaustive()
    }
}

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

    /// The origin location of the event, when present.
    #[must_use]
    pub fn from(&self) -> Option<&Location> {
        match self {
            RouterEvent::BeforeNavigate { from, .. }
            | RouterEvent::BeforeLoad { from, .. }
            | RouterEvent::Load { from, .. }
            | RouterEvent::BeforeRouteMount { from, .. }
            | RouterEvent::Resolved { from, .. }
            | RouterEvent::Rendered { from, .. } => from.as_ref(),
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

impl<T: std::fmt::Debug> std::fmt::Debug for LoaderOutcome<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderOutcome::Data(data) => f.debug_tuple("Data").field(data).finish(),
            LoaderOutcome::Redirect(redirect) => f.debug_tuple("Redirect").field(redirect).finish(),
            LoaderOutcome::NotFound(not_found) => {
                f.debug_tuple("NotFound").field(not_found).finish()
            }
        }
    }
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

/// Insertion-ordered, capacity-bounded scroll-position memory shared by
/// [`ScrollRestoration`](crate::ScrollRestoration) nodes.
#[derive(Clone)]
pub struct ScrollPositions {
    inner: Arc<Mutex<ScrollPositionsInner>>,
}

#[derive(Default)]
struct ScrollPositionsInner {
    map: FxHashMap<String, f32>,
    order: VecDeque<String>,
    capacity: usize,
}

impl ScrollPositions {
    /// Creates an empty store with the default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ScrollPositionsInner {
                capacity: DEFAULT_SCROLL_CACHE_CAPACITY,
                ..ScrollPositionsInner::default()
            })),
        }
    }

    /// Build a stable, insertion-order-independent key for a location.
    #[must_use]
    pub fn key(pathname: &str, search: &serde_json::Value) -> String {
        let search_key = canonical_search_key(search);
        if search_key.is_empty() {
            pathname.to_string()
        } else {
            format!("{pathname}?{search_key}")
        }
    }

    /// Bound how many locations remember their scroll offset.
    pub fn set_capacity(&self, capacity: usize) {
        self.inner.lock().capacity = capacity;
    }

    /// The current capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.lock().capacity
    }

    /// Save the current scroll offset for a key.
    ///
    /// Eviction is O(1): the insertion order lives in a `VecDeque` ring
    /// buffer, so overflowing the capacity pops the oldest key from the
    /// front instead of shifting the whole vector.
    pub fn insert(&self, key: String, offset: f32) {
        let mut inner = self.inner.lock();
        if !inner.map.contains_key(&key) {
            inner.order.push_back(key.clone());
            while inner.order.len() > inner.capacity {
                if let Some(evicted) = inner.order.pop_front() {
                    inner.map.remove(&evicted);
                }
            }
        }
        inner.map.insert(key, offset);
    }

    /// Get a previously saved scroll offset for a key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<f32> {
        self.inner.lock().map.get(key).copied()
    }
}

impl Default for ScrollPositions {
    fn default() -> Self {
        Self::new()
    }
}

/// Default upper bound on remembered scroll positions. Bounded memory even
/// for apps that wander through thousands of dynamic routes.
pub const DEFAULT_SCROLL_CACHE_CAPACITY: usize = 128;

/// Build a stable, insertion-order-independent string for a search value so
/// `?a=1&b=2` and `?b=2&a=1` share one scroll entry. Nested JSON is handled
/// by serializing with object keys sorted. Builds the key in a single
/// `String` buffer instead of a `Vec<String>` of `format!` fragments.
#[must_use]
pub fn canonical_search_key(search: &serde_json::Value) -> String {
    match search {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => {
            let mut pairs: Vec<(String, String)> = map
                .iter()
                .map(|(k, v)| (k.clone(), canonical_search_key(v)))
                .collect();
            pairs.sort();
            let mut out = String::new();
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push('&');
                }
                out.push_str(k);
                out.push('=');
                out.push_str(v);
            }
            out
        }
        other => other.to_string(),
    }
}

/// The full router state. Plain data: shared through [`RouterHandle`].
pub struct RouterState {
    pub(crate) core: RouterCore,
    pub route_tree: Arc<RouteTree>,
    pub current_match: Option<(HashMap<String, String>, Arc<RouteNode>)>,
    pub pending_navigation: Option<Location>,
    pub blockers: HashMap<BlockerId, Blocker>,
    pub scroll_restoration: bool,
    pub default_view_transition: Option<ViewTransitionOptions>,
    pub current_base: Option<String>,
    events: Vec<Arc<dyn Fn(RouterEvent) + Send + Sync>>,
    next_blocker_id: BlockerId,
    pub query_client: QueryClient,
    loader_factories: FxHashMap<String, LoaderFactory>,
    context_cache: FxHashMap<String, serde_json::Value>,
    /// Framework-agnostic host seam (repaint + UI-thread spawn).
    pub host: RouterHost,
    /// Opaque window identity for `navi-core`'s context tree.
    pub window_key: WindowKey,
    /// Opaque root-view identifier assigned by the embedding application.
    pub root_view: Option<u64>,
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
    /// Scroll-offset memory shared with restoration nodes.
    pub scroll_positions: ScrollPositions,
    /// The router event log (devtools timeline source).
    pub(crate) event_log: crate::event_bus::EventLog,
    /// Params of the current match serialized once per navigation; per-frame
    /// `use_params!` calls clone this instead of re-serializing the map.
    current_params_json: Option<serde_json::Value>,
    /// Lazily-built memos of the loader [`QueryKey`] (unit-params and
    /// map-params variants): initialized by the first per-frame
    /// `get_loader_data`/`get_loader_state` call after a navigation, taken
    /// (invalidated) on the next navigation. `get_loader_data` runs under a
    /// read guard, so the memo must be `&self`-initializable.
    loader_key_cache_unit: OnceCell<(String, QueryKey)>,
    loader_key_cache_params: OnceCell<(String, QueryKey)>,
}

/// A cheap handle to the shared router state + event log.
///
/// Cloning is free (Arc); navigation paths, components, devtools and tests
/// all drive the router through this handle.
#[derive(Clone)]
pub struct RouterHandle {
    state: Arc<RwLock<RouterState>>,
}

impl RouterHandle {
    /// The router event log (devtools timeline source).
    #[must_use]
    pub fn events(&self) -> crate::event_bus::EventLog {
        self.state.read().event_log.clone()
    }
}

thread_local! {
    static CURRENT: std::cell::RefCell<Option<RouterHandle>> =
        const { std::cell::RefCell::new(None) };
}

impl RouterHandle {
    /// Creates a router handle with the given initial location and options.
    #[must_use]
    pub fn new_with_options(
        initial: Location,
        window_key: WindowKey,
        route_tree: Arc<RouteTree>,
        options: RouterOptions,
        host: RouterHost,
    ) -> Self {
        let state = RouterState::new_with_options(initial, window_key, route_tree, options, host);
        // The handle is deliberately shared as `Arc` even though the state
        // hosts `Rc`-based UI seams: the router is bound to the window's
        // UI thread, and cross-thread use is guarded by `SendWrapper` at
        // the embedding layer.
        #[expect(clippy::arc_with_non_send_sync)]
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Creates a router handle with default options.
    #[must_use]
    pub fn new(
        initial: Location,
        window_key: WindowKey,
        route_tree: Arc<RouteTree>,
        host: RouterHost,
    ) -> Self {
        Self::new_with_options(
            initial,
            window_key,
            route_tree,
            RouterOptions::default(),
            host,
        )
    }

    /// Installs this handle as the thread-current router (used by hooks
    /// macros and the components' no-context constructors).
    pub fn install_current(&self) {
        CURRENT.with(|slot| *slot.borrow_mut() = Some(self.clone()));
    }

    /// Removes the thread-current router when it is this handle.
    pub fn uninstall_current(&self) {
        CURRENT.with(|slot| {
            let is_current = slot
                .borrow()
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(&current.state, &self.state));
            if is_current {
                *slot.borrow_mut() = None;
            }
        });
    }

    /// The thread-current router handle.
    ///
    /// # Panics
    /// Panics when no router was installed on this thread (parity with the
    /// classic global accessor).
    #[must_use]
    pub fn current() -> Self {
        Self::try_current().unwrap_or_else(|| {
            panic!("RouterHandle::current() called before RouterProvider::install")
        })
    }

    /// The thread-current router handle, when one is installed.
    #[must_use]
    pub fn try_current() -> Option<Self> {
        CURRENT.with(|slot| slot.borrow().clone())
    }

    /// Read access to the state.
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, RouterState> {
        self.state.read()
    }

    /// Mutable access to the state.
    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, RouterState> {
        self.state.write()
    }

    /// Runs `f` with mutable access to the state.
    pub fn update<R>(&self, f: impl FnOnce(&mut RouterState) -> R) -> R {
        let mut guard = self.state.write();
        f(&mut guard)
    }

    /// Requests a repaint through the host seam.
    pub fn refresh(&self) {
        (self.state.read().host.refresh)();
    }

    /// The host seam.
    #[must_use]
    pub fn host(&self) -> RouterHost {
        self.state.read().host.clone()
    }
}

/// Build the canonical cache key for a route loader from parts precomputed
/// at navigation time ([`RouterState::refresh_match_caches`]). Shared by
/// [`RouterState::get_loader_data`] and [`RouterState::get_loader_state`].
fn build_loader_key(route_id: &str, params_str: &str, deps_str: Option<&str>) -> QueryKey {
    let mut key_builder = QueryKey::new("navi_loader")
        .with("route", route_id)
        .with("params", params_str);
    if let Some(deps) = deps_str {
        key_builder = key_builder.with("deps", deps);
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
        window_key: WindowKey,
        route_tree: Arc<RouteTree>,
        host: RouterHost,
    ) -> Self {
        Self::new_with_options(
            initial,
            window_key,
            route_tree,
            RouterOptions::default(),
            host,
        )
    }

    #[must_use]
    pub fn new_with_options(
        initial: Location,
        window_key: WindowKey,
        route_tree: Arc<RouteTree>,
        options: RouterOptions,
        host: RouterHost,
    ) -> Self {
        let core = RouterCore::new(initial, Arc::clone(&route_tree));
        let current_match = core.current_match.clone();
        let mut state = Self {
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
            loader_factories: FxHashMap::default(),
            context_cache: FxHashMap::default(),
            host,
            window_key,
            root_view: None,
            not_found_mode: options.not_found_mode,
            not_found_data: None,
            default_pending_ms: options.default_pending_ms,
            default_pending_min_ms: options.default_pending_min_ms,
            navigating_not_found: false,
            scroll_reset_pending: false,
            scroll_positions: ScrollPositions::new(),
            event_log: crate::event_bus::EventLog::new(),
            current_params_json: None,
            loader_key_cache_unit: OnceCell::new(),
            loader_key_cache_params: OnceCell::new(),
        };
        state.refresh_match_caches();
        state
    }

    // History delegates. The core re-matches on traversal; we mirror its
    // fresh `current_match` into the state copy the UI reads.
    pub fn back(&mut self) -> bool {
        let moved = self.core.back();
        if moved {
            self.current_match = self.core.current_match.clone();
            self.refresh_match_caches();
        }
        moved
    }

    pub fn forward(&mut self) -> bool {
        let moved = self.core.forward();
        if moved {
            self.current_match = self.core.current_match.clone();
            self.refresh_match_caches();
        }
        moved
    }

    pub fn go(&mut self, delta: isize) {
        self.core.go(delta);
        self.current_match = self.core.current_match.clone();
        self.refresh_match_caches();
    }

    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.core.history().can_go_back()
    }

    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.core.history().can_go_forward()
    }

    pub fn set_root_view(&mut self, view_id: u64) {
        self.root_view = Some(view_id);
    }

    #[must_use]
    pub fn root_view(&self) -> Option<u64> {
        self.root_view
    }

    /// Send an event to subscribers and to the global event log (devtools).
    fn dispatch(&self, event: &RouterEvent) {
        self.event_log.push(event.clone());
        self.emit(event);
    }

    pub fn navigate(&mut self, loc: Location, options: &NavigateOptions) {
        log::debug!("navigate called: {}", loc.pathname);

        self.dispatch(&RouterEvent::BeforeNavigate {
            from: Some(self.current_location()),
            to: loc.clone(),
        });

        // --- Blockers (async handling) ---
        if !options.ignore_blocker && !self.blockers.is_empty() {
            let current = self.current_location();
            let blockers: Vec<Blocker> = self.blockers.values().cloned().collect();
            let loc_clone = loc.clone();
            let options_clone = options.clone();
            let handle = current_handle_of(self);
            self.host.spawn(Box::pin(async move {
                let mut allow = true;
                for blocker in &blockers {
                    if !blocker.should_allow(&current, &loc_clone).await {
                        allow = false;
                        break;
                    }
                }
                if allow {
                    handle.update(|state| state.commit_navigation(loc_clone, &options_clone));
                    handle.refresh();
                } else {
                    handle.update(|state| state.pending_navigation = Some(loc_clone));
                    handle.refresh();
                }
            }));
            return;
        }

        self.commit_navigation(loc, options);
    }

    fn commit_navigation(&mut self, loc: Location, options: &NavigateOptions) {
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
            self.refresh_match_caches();
            self.handle_navigation_effects(effects);
            self.dispatch(&RouterEvent::Resolved { from, to: loc });
            return;
        };

        let before_load_fns: Vec<crate::route_tree::BeforeLoadFn> = self
            .route_tree
            .ancestors(&matched_node.id)
            .iter()
            .filter_map(|node| node.before_load.clone())
            .collect();

        self.dispatch(&RouterEvent::BeforeLoad {
            from: from.clone(),
            to: loc.clone(),
        });

        if !before_load_fns.is_empty() {
            let loc_clone = loc.clone();
            let options_clone = options.clone();
            let handle = current_handle_of(self);

            self.host.spawn(Box::pin(async move {
                for before_load in before_load_fns {
                    let ctx = crate::route_tree::BeforeLoadContext {
                        params: HashMap::new(),
                        search: loc_clone.search.clone(),
                        location: loc_clone.clone(),
                    };
                    match before_load(ctx).await {
                        crate::route_tree::BeforeLoadResult::Ok => {}
                        crate::route_tree::BeforeLoadResult::Redirect(redirect) => {
                            let nav = crate::Navigator::from_handle(&handle);
                            nav.push_location(Location::new(&redirect.to));
                            return;
                        }
                        crate::route_tree::BeforeLoadResult::NotFound(not_found) => {
                            // No nested `handle.update` here: parking_lot
                            // locks are not reentrant, so the fallback
                            // navigation must run after the update closes.
                            let mut navigate_not_found = false;
                            handle.update(|state| {
                                state.not_found_data = not_found.data;
                                navigate_not_found =
                                    state.current_location().pathname != NOT_FOUND_PATH;
                            });
                            if navigate_not_found {
                                let nav = crate::Navigator::from_handle(&handle);
                                nav.push(NOT_FOUND_PATH);
                            }
                            return;
                        }
                    }
                }
                // All before_load passed, proceed with core navigation
                handle.update(|state| {
                    let effects =
                        state
                            .core
                            .navigate_with(loc_clone, &options_clone, matched.clone());
                    state.current_match = state.core.current_match.clone();
                    state.refresh_match_caches();
                    state.handle_navigation_effects(effects);
                });
            }));
            return;
        }

        // No before_load hooks, direct navigation
        let effects = self.core.navigate_with(loc.clone(), options, matched);
        self.current_match = self.core.current_match.clone();
        self.refresh_match_caches();
        self.handle_navigation_effects(effects);
        self.dispatch(&RouterEvent::Resolved { from, to: loc });
    }

    fn handle_navigation_effects(&mut self, effects: Vec<NavigationEffect>) {
        for effect in effects {
            match effect {
                NavigationEffect::SpawnLoader { route_id, params } => {
                    if let Some(factory) = self.loader_factories.get(&route_id) {
                        let query = factory(&params);
                        let key = query.key.clone();
                        let client = self.query_client.clone();
                        let fetch_fn = query.fetch_fn.clone();
                        let query_options = query.options.clone();
                        let handle = current_handle_of(self);

                        self.host.spawn(Box::pin(async move {
                            match (fetch_fn)().await {
                                Ok(outcome) => match outcome {
                                    LoaderOutcome::Data(data) => {
                                        client.set_query_data(&key, data, query_options.clone());
                                        handle.refresh();
                                    }
                                    LoaderOutcome::Redirect(redirect) => {
                                        let nav = crate::Navigator::from_handle(&handle);
                                        nav.push_location(Location::new(&redirect.to));
                                    }
                                    LoaderOutcome::NotFound(not_found) => {
                                        // No nested `handle.update` (locks are
                                        // not reentrant): record the data, then
                                        // navigate after the update closes.
                                        let mut navigate_not_found = false;
                                        handle.update(|state| {
                                            state.not_found_data = not_found.data;
                                            navigate_not_found =
                                                state.current_location().pathname != NOT_FOUND_PATH;
                                        });
                                        if navigate_not_found {
                                            let nav = crate::Navigator::from_handle(&handle);
                                            nav.push(NOT_FOUND_PATH);
                                        }
                                        handle.refresh();
                                    }
                                },
                                Err(e) => {
                                    log::error!("Loader error for {route_id}: {e:?}");
                                    handle.refresh();
                                }
                            }
                        }));
                    }
                }
                NavigationEffect::Redirect { to, replace } => {
                    // Direct call: we are already inside `update`, so a
                    // nested handle `update` is unnecessary.
                    self.navigate(
                        Location::new(&to),
                        &NavigateOptions {
                            replace,
                            ..NavigateOptions::default()
                        },
                    );
                    return;
                }
                NavigationEffect::NotFound { data } => {
                    self.not_found_data = data;
                    // Direct call (no nested update) and a recursion guard
                    // for trees without a dedicated `/404` route: that
                    // navigation itself produces another NotFound effect.
                    if !self.navigating_not_found {
                        self.navigating_not_found = true;
                        self.navigate(Location::new(NOT_FOUND_PATH), &NavigateOptions::default());
                        self.navigating_not_found = false;
                    }
                    return;
                }
                NavigationEffect::NotifyListeners => {
                    self.host.refresh();
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
    pub fn transact(&mut self, ops: &[NavOp]) {
        let has_moves = ops
            .iter()
            .any(|op| matches!(op, NavOp::Push(_) | NavOp::Replace(_)));
        if has_moves && !self.blockers.is_empty() {
            for op in ops {
                match op {
                    NavOp::Push(loc) => {
                        self.navigate(loc.clone(), &NavigateOptions::default());
                    }
                    NavOp::Replace(loc) => {
                        self.navigate(
                            loc.clone(),
                            &NavigateOptions {
                                replace: true,
                                ..NavigateOptions::default()
                            },
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
            self.dispatch(&RouterEvent::BeforeNavigate {
                from: from.clone(),
                to: to.clone(),
            });
        }
        let effects = self.core.transact(ops);
        self.current_match = self.core.current_match.clone();
        self.refresh_match_caches();
        self.handle_navigation_effects(effects);
        if let Some(to) = to {
            self.dispatch(&RouterEvent::Resolved { from, to });
        }
    }

    /// Consume the one-shot flag set by a navigation with
    /// `reset_scroll: Some(true)`. [`ScrollRestoration`](crate::ScrollRestoration)
    /// uses it to skip restoring the saved offset exactly once.
    pub fn take_scroll_reset(&mut self) -> bool {
        std::mem::take(&mut self.scroll_reset_pending)
    }

    pub fn preload_location(&mut self, loc: &Location) {
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
            // Preloads fire and forget on the quoin runtime seam.
            self.host.spawn(Box::pin(async move {
                match (fetch_fn)().await {
                    Ok(LoaderOutcome::Data(data)) => {
                        client.set_query_data(&key, data, options);
                    }
                    Ok(LoaderOutcome::Redirect(_) | LoaderOutcome::NotFound(_)) => {}
                    Err(e) => {
                        log::error!("Preload error for {node_id}: {e}");
                    }
                }
            }));
        }
    }

    #[must_use]
    pub fn current_location(&self) -> Location {
        self.core.current_location()
    }

    /// The current pathname by reference (no `Location` clone). Use this in
    /// per-frame code that only needs the path (active-link checks, keys).
    #[must_use]
    pub fn current_pathname(&self) -> &str {
        self.core.current_location_ref().pathname.as_str()
    }

    /// The current hash by reference (no `Location` clone).
    #[must_use]
    pub fn current_hash(&self) -> &str {
        self.core.current_location_ref().hash.as_str()
    }

    /// The current search params by reference (no `Location` clone).
    #[must_use]
    pub fn current_search_value(&self) -> &serde_json::Value {
        &self.core.current_location_ref().search
    }

    /// The params of the current match, serialized once at navigation time.
    /// `None` when no route is matched.
    #[must_use]
    pub fn current_params_json(&self) -> Option<&serde_json::Value> {
        self.current_params_json.as_ref()
    }

    pub fn emit(&self, event: &RouterEvent) {
        for listener in &self.events {
            listener(event.clone());
        }
    }

    pub fn subscribe<F: Fn(RouterEvent) + Send + Sync + 'static>(&mut self, f: F) {
        self.events.push(Arc::new(f));
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

    pub fn proceed(&mut self) {
        if let Some(loc) = self.pending_navigation.take() {
            self.navigate(
                loc,
                &NavigateOptions {
                    ignore_blocker: true,
                    ..NavigateOptions::default()
                },
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
        let (_, node) = self.current_match.as_ref()?;
        if node.id != R::name() {
            return None;
        }
        let unit = std::any::TypeId::of::<R::Params>() == std::any::TypeId::of::<()>();
        let cache = if unit {
            &self.loader_key_cache_unit
        } else {
            &self.loader_key_cache_params
        };
        // Memo hit: the key was built by the first hook call after the
        // navigation committed. Cheap string compare, no allocation.
        if let Some((route_id, key)) = cache.get()
            && route_id == &node.id
        {
            return Some(key.clone());
        }
        // Cold path (once per navigation per variant): build from the cached
        // params JSON instead of re-serializing the params map.
        let node_id = node.id.clone();
        let (_, key) = cache.get_or_init(|| self.build_loader_key_cached(node_id, unit));
        Some(key.clone())
    }

    /// Build the loader key for the current match from the cached params
    /// JSON instead of re-serializing the params map.
    fn build_loader_key_cached(&self, route_id: String, unit: bool) -> (String, QueryKey) {
        let params_str = if unit {
            "null".to_string()
        } else {
            self.current_params_json.as_ref().map_or_else(
                || "null".to_string(),
                |j| serde_json::to_string(j).unwrap_or_else(|_| "null".to_string()),
            )
        };
        let deps_str = self.current_match.as_ref().and_then(|(_, node)| {
            node.loader_deps.as_ref().and_then(|f| {
                let deps_json = f(&self.core.history().current_ref().search);
                if deps_json.is_null() {
                    None
                } else {
                    Some(serde_json::to_string(&deps_json).unwrap_or_default())
                }
            })
        });
        let key = build_loader_key(&route_id, &params_str, deps_str.as_deref());
        (route_id, key)
    }

    /// Reset the per-navigation caches after `current_match` changed:
    /// re-serialize the params JSON (for `use_params!`) and drop the loader
    /// key memos (rebuilt lazily by the first per-frame hook call).
    fn refresh_match_caches(&mut self) {
        self.current_params_json = self
            .current_match
            .as_ref()
            .and_then(|(params, _node)| serde_json::to_value(params).ok());
        self.loader_key_cache_unit.take();
        self.loader_key_cache_params.take();
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
}

/// Extracts the [`RouterHandle`] sharing `state`. Used inside state methods
/// that spawn async continuations: those continuations must mutate the same
/// shared state, and the handle is the cheapest way to carry it.
fn current_handle_of(_state: &RouterState) -> RouterHandle {
    RouterHandle::try_current().unwrap_or_else(|| {
        panic!(
            "router async continuation requires the RouterHandle to be installed (RouterProvider::install or RouterHandle::install_current)"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop_host() -> RouterHost {
        RouterHost {
            refresh: Rc::new(|| {}),
            spawn: Rc::new(|_future: LocalFuture| {}),
        }
    }

    fn state_with(tree: RouteTree) -> RouterState {
        RouterState::new(
            Location::new("/"),
            navi_core::WindowKey(1),
            Arc::new(tree),
            noop_host(),
        )
    }

    /// `NavigationEffect::Redirect` is part of the adapter contract but
    /// `RouterCore` never emits it; drive the effect arm directly so the
    /// behavior (navigate to the redirect target, then stop) stays pinned.
    #[test]
    fn handle_navigation_effects_redirect_effect_navigates() {
        let mut state = state_with(RouteTree::new());
        state.handle_navigation_effects(vec![NavigationEffect::Redirect {
            to: "/somewhere".to_string(),
            replace: false,
        }]);
        // The redirect target is unmatched: not-found handling kicked in.
        assert!(state.not_found_data.is_none() || state.current_location().pathname != "/");
    }

    #[test]
    fn router_host_debug_is_opaque() {
        let host = noop_host();
        let dbg = format!("{host:?}");
        assert!(dbg.contains("RouterHost"));
    }

    #[test]
    fn loader_outcome_equality_rules() {
        let d1: LoaderOutcome<AnyData> = LoaderOutcome::Data(AnyData::new(1_u8));
        let d2: LoaderOutcome<AnyData> = LoaderOutcome::Data(AnyData::new(1_u8));
        // Type-erased data never compares equal (AnyData PartialEq).
        assert!(!(d1 == d2));
        let r1: LoaderOutcome<AnyData> = LoaderOutcome::Redirect(crate::redirect("/a"));
        let r2: LoaderOutcome<AnyData> = LoaderOutcome::Redirect(crate::redirect("/a"));
        let r3: LoaderOutcome<AnyData> = LoaderOutcome::Redirect(crate::redirect("/b"));
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
        let nf1: LoaderOutcome<AnyData> =
            LoaderOutcome::NotFound(crate::not_found().with_route_id("r1"));
        let nf2: LoaderOutcome<AnyData> =
            LoaderOutcome::NotFound(crate::not_found().with_route_id("r1"));
        let nf3: LoaderOutcome<AnyData> =
            LoaderOutcome::NotFound(crate::not_found().with_route_id("r2"));
        assert_eq!(nf1, nf2);
        assert_ne!(nf1, nf3);
        assert_ne!(d1, nf1);
    }

    #[test]
    fn router_event_from_accessor_with_origin() {
        let event = RouterEvent::Resolved {
            from: Some(Location::new("/a")),
            to: Location::new("/b"),
        };
        assert_eq!(event.from().map(|l| l.pathname.clone()), Some("/a".into()));
        assert_eq!(event.to().map(|l| l.pathname.clone()), Some("/b".into()));
        let event = RouterEvent::Rendered {
            from: None,
            to: Location::new("/b"),
        };
        assert!(event.from().is_none());
    }

    #[test]
    #[should_panic(expected = "router async continuation requires the RouterHandle")]
    fn blocked_navigation_without_installed_handle_panics() {
        let mut state = state_with(RouteTree::new());
        state.add_blocker(Blocker::new_sync(|_, _| false));
        // No `RouterHandle` installed on this thread: the blocker
        // continuation cannot be wired up, which must panic loudly.
        state.navigate(Location::new("/"), &NavigateOptions::default());
    }

    #[test]
    fn router_options_defaults_match_docs() {
        let options = RouterOptions::default();
        assert_eq!(options.default_pending_ms, 1000);
        assert_eq!(options.default_pending_min_ms, 500);
        assert!(matches!(options.not_found_mode, NotFoundMode::Root));
    }
}
