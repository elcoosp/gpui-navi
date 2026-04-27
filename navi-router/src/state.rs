use crate::{
    Blocker, BlockerId, Location, NavigateOptions, NotFound, Redirect,
    RouteNode, RouteTree, ViewTransitionOptions,
};
use gpui::{AnyWindowHandle, App, BorrowAppContext, EntityId, Global, WindowId};
use navi_router_core::{NavigationEffect, RouterCore};
use rs_query::{Query, QueryClient, QueryKey};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterEvent {
    BeforeNavigate { from: Option<Location>, to: Location },
    BeforeLoad { from: Option<Location>, to: Location },
    Load { from: Option<Location>, to: Location },
    BeforeRouteMount { from: Option<Location>, to: Location },
    Resolved { from: Option<Location>, to: Location },
    Rendered { from: Option<Location>, to: Location },
}

pub trait RouteDef: 'static {
    type Params: Clone + std::fmt::Debug + DeserializeOwned + 'static;
    type Search: Clone + std::fmt::Debug + 'static;
    type LoaderData: Clone + std::fmt::Debug + Send + Sync + 'static;
    fn path() -> &'static str;
    fn name() -> &'static str;
}

#[derive(Clone)]
pub struct AnyData(pub Arc<dyn std::any::Any + Send + Sync>);

impl PartialEq for AnyData {
    fn eq(&self, _other: &Self) -> bool { false }
}

impl std::fmt::Debug for AnyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyData").finish()
    }
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

type LoaderFactory = Arc<dyn Fn(&HashMap<String, String>) -> Query<LoaderOutcome<AnyData>> + Send + Sync>;

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
    pub current_match: Option<(HashMap<String, String>, RouteNode)>,
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
}

impl Global for RouterState {}

impl RouterState {
    pub fn new(
        initial: Location,
        window_id: WindowId,
        window_handle: AnyWindowHandle,
        route_tree: Rc<RouteTree>,
    ) -> Self {
        Self::new_with_options(initial, window_id, window_handle, route_tree, RouterOptions::default())
    }

    pub fn new_with_options(
        initial: Location,
        _window_id: WindowId,
        window_handle: AnyWindowHandle,
        route_tree: Rc<RouteTree>,
        options: RouterOptions,
    ) -> Self {
        let core = RouterCore::new(initial.clone(), (*route_tree).clone());
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
        }
    }

    // History delegates
    pub fn back(&mut self) -> bool { self.core.back() }
    pub fn forward(&mut self) -> bool { self.core.forward() }
    pub fn go(&mut self, delta: isize) { self.core.go(delta); }
    pub fn can_go_back(&self) -> bool { self.core.history().can_go_back() }
    pub fn can_go_forward(&self) -> bool { self.core.history().can_go_forward() }

    pub fn set_root_view(&mut self, view_id: EntityId) {
        self.root_view = Some(view_id);
    }

    pub fn navigate(&mut self, loc: Location, options: NavigateOptions, cx: &mut App) {
        log::debug!("navigate called: {}", loc.pathname);

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
                        let _ = cx.update(|cx| {
                            RouterState::update(cx, |state, cx| {
                                if allow {
                                    state.commit_navigation(loc_clone, options_clone, cx);
                                } else {
                                    state.pending_navigation = Some(loc_clone);
                                }
                            });
                        });
                    }
                }
            }).detach();
            return;
        }

        self.commit_navigation(loc, options, cx);
    }

    fn commit_navigation(&mut self, loc: Location, options: NavigateOptions, cx: &mut App) {
        // --- before_load hooks ---
        let matched_node = match self.route_tree.match_path(&loc.pathname) {
            Some((_params, node)) => node.clone(),
            None => {
                // No match -> not found
                let effects = self.core.navigate(loc.clone(), options.clone());
                self.current_match = self.core.current_match.clone();
                self.handle_navigation_effects(effects, cx);
                return;
            }
        };

        let before_load_fns: Vec<(String, crate::route_tree::BeforeLoadFn)> = self
            .route_tree
            .ancestors(&matched_node.id)
            .iter()
            .filter_map(|node| node.before_load.as_ref().map(|f| (node.id.clone(), f.clone())))
            .collect();

        if !before_load_fns.is_empty() {
            let window_handle = self.window_handle;
            let loc_clone = loc.clone();
            let before_load_fns = before_load_fns.clone();

            cx.spawn(move |cx: &mut gpui::AsyncApp| {
                let cx = cx.clone();
                async move {
                    for (_route_id, before_load) in before_load_fns {
                        let ctx = crate::route_tree::BeforeLoadContext {
                            params: HashMap::new(),
                            search: loc_clone.search.clone(),
                            location: loc_clone.clone(),
                        };
                        match before_load(ctx).await {
                            crate::route_tree::BeforeLoadResult::Ok => continue,
                            crate::route_tree::BeforeLoadResult::Redirect(redirect) => {
                                let _ = cx.update(|cx| {
                                    let nav = crate::Navigator::new(window_handle);
                                    nav.push_location(Location::new(&redirect.to), cx);
                                });
                                return;
                            }
                            crate::route_tree::BeforeLoadResult::NotFound(not_found) => {
                                let _ = cx.update(|cx| {
                                    RouterState::update(cx, |state, cx| {
                                        state.not_found_data = not_found.data;
                                        let nav = crate::Navigator::new(state.window_handle);
                                        nav.push("/404", cx);
                                    });
                                });
                                return;
                            }
                        }
                    }
                    // All before_load passed, proceed with core navigation
                    let _ = cx.update(|cx| {
                        RouterState::update(cx, |state, cx| {
                            let effects = state.core.navigate(loc_clone, options);
                            state.current_match = state.core.current_match.clone();
                            state.handle_navigation_effects(effects, cx);
                        });
                    });
                }
            }).detach();
            return;
        }

        // No before_load hooks, direct navigation
        let effects = self.core.navigate(loc, options);
        self.current_match = self.core.current_match.clone();
        self.handle_navigation_effects(effects, cx);
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
                                            client.set_query_data(&key, data, query_options.clone());
                                            let _ = cx.update(|cx| {
                                                RouterState::update(cx, |_, cx| cx.refresh_windows());
                                                let _ = window_handle.update(cx, |_, window, _| window.refresh());
                                            });
                                        }
                                        LoaderOutcome::Redirect(redirect) => {
                                            let _ = cx.update(|cx| {
                                                let nav = crate::Navigator::new(window_handle);
                                                nav.push_location(Location::new(&redirect.to), cx);
                                            });
                                        }
                                        LoaderOutcome::NotFound(not_found) => {
                                            let _ = cx.update(|cx| {
                                                RouterState::update(cx, |state, cx| {
                                                    state.not_found_data = not_found.data;
                                                    let nav = crate::Navigator::new(state.window_handle);
                                                    nav.push("/404", cx);
                                                });
                                            });
                                        }
                                    },
                                    Err(e) => {
                                        log::error!("Loader error for {}: {:?}", route_id, e);
                                        let _ = cx.update(|cx| {
                                            RouterState::update(cx, |_, cx| cx.refresh_windows());
                                            let _ = window_handle.update(cx, |_, window, _| window.refresh());
                                        });
                                    }
                                }
                            }
                        })
                        .detach();
                    }
                }
                NavigationEffect::Redirect { to, replace } => {
                    let nav = crate::Navigator::new(self.window_handle);
                    if replace {
                        nav.replace(to, cx);
                    } else {
                        nav.push(to, cx);
                    }
                    return;
                }
                NavigationEffect::NotFound { data } => {
                    self.not_found_data = data;
                    let nav = crate::Navigator::new(self.window_handle);
                    nav.push("/404", cx);
                    return;
                }
                NavigationEffect::NotifyListeners => {
                    cx.refresh_windows();
                    let _ = self.window_handle.update(cx, |_, window, _| window.refresh());
                }
            }
        }
    }

    pub fn preload_location(&mut self, loc: Location, cx: &mut App) {
        if let Some((params, node)) = self.route_tree.match_path(&loc.pathname)
            && node.has_loader
            && let Some(factory) = self.loader_factories.get(&node.id)
        {
            let stale_time = node.loader_stale_time.unwrap_or(std::time::Duration::ZERO);
            let gc_time = node.loader_gc_time.unwrap_or(std::time::Duration::from_secs(300));
            let query = factory(&params).stale_time(stale_time).gc_time(gc_time);
            let key = query.key.clone();
            let client = self.query_client.clone();
            let fetch_fn = query.fetch_fn.clone();
            let options = query.options.clone();
            let node_id = node.id.clone();
            cx.spawn(|_cx: &mut gpui::AsyncApp| {
                async move {
                    match (fetch_fn)().await {
                        Ok(LoaderOutcome::Data(data)) => {
                            client.set_query_data(&key, data, options);
                        }
                        Ok(LoaderOutcome::Redirect(_)) | Ok(LoaderOutcome::NotFound(_)) => {}
                        Err(e) => {
                            log::error!("Preload error for {}: {}", node_id, e);
                        }
                    }
                }
            })
            .detach();
        }
    }

    pub fn current_location(&self) -> Location {
        self.core.current_location()
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
                NavigateOptions { ignore_blocker: true, ..Default::default() },
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

    pub fn register_loader_factory(&mut self, route_id: &str, factory: LoaderFactory) {
        self.loader_factories.insert(route_id.to_string(), factory);
    }

    pub fn get_loader_data<R: crate::RouteDef>(&self) -> Option<R::LoaderData> {
        let (params, node) = self.current_match.as_ref()?;
        if node.id != R::name() {
            return None;
        }
        let deps_json = node
            .loader_deps
            .as_ref()
            .map(|f| f(&self.current_location().search))
            .unwrap_or(serde_json::Value::Null);
        let params_str = if std::any::TypeId::of::<R::Params>() == std::any::TypeId::of::<()>() {
            "null".to_string()
        } else {
            serde_json::to_string(params).unwrap_or_else(|_| "null".to_string())
        };
        let mut key_builder = QueryKey::new("navi_loader")
            .with("route", node.id.as_str())
            .with("params", params_str);
        if !deps_json.is_null() {
            key_builder = key_builder.with("deps", serde_json::to_string(&deps_json).unwrap_or_default());
        }
        let key = key_builder;
        let any_data: AnyData = self.query_client.get_query_data(&key)?;
        let arc_data = any_data.0.downcast_ref::<Arc<R::LoaderData>>()?.clone();
        Some((*arc_data).clone())
    }

    pub fn get_loader_state<R: crate::RouteDef>(&self) -> LoaderState {
        let (params, node) = match &self.current_match {
            Some(m) => m,
            None => return LoaderState::Idle,
        };
        if node.id != R::name() {
            return LoaderState::Idle;
        }
        let deps_json = node
            .loader_deps
            .as_ref()
            .map(|f| f(&self.current_location().search))
            .unwrap_or(serde_json::Value::Null);
        let params_str = if std::any::TypeId::of::<R::Params>() == std::any::TypeId::of::<()>() {
            "null".to_string()
        } else {
            serde_json::to_string(params).unwrap_or_else(|_| "null".to_string())
        };
        let mut key_builder = QueryKey::new("navi_loader")
            .with("route", node.id.as_str())
            .with("params", params_str);
        if !deps_json.is_null() {
            key_builder = key_builder.with("deps", serde_json::to_string(&deps_json).unwrap_or_default());
        }
        let key = key_builder;
        if self.query_client.is_in_flight(&key) {
            LoaderState::Loading
        } else if self.query_client.get_query_data::<AnyData>(&key).is_some() {
            LoaderState::Ready
        } else {
            LoaderState::Idle
        }
    }

    pub fn get_route_context<R: crate::RouteDef>(&self) -> Option<serde_json::Value> {
        let (_, node) = self.current_match.as_ref()?;
        if node.id != R::name() {
            return None;
        }
        self.context_cache.get(&node.id).cloned()
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

    pub fn has_pending_loader(&self) -> bool {
        self.query_client.is_fetching()
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }
}
