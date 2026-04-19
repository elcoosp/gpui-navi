// navi-router/src/state.rs
use crate::blocker::{Blocker, BlockerId};
use crate::event_bus::push_event;
use crate::history::History;
use crate::location::{Location, NavigateOptions, ViewTransitionOptions};
use crate::redirect::{NotFound, Redirect};
use crate::route_tree::{BeforeLoadContext, BeforeLoadResult, RouteNode, RouteTree};
use gpui::{AnyWindowHandle, App, BorrowAppContext, EntityId, Global, WindowId};
use rs_query::{Query, QueryClient, QueryKey};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

/// Events emitted by the router during navigation lifecycle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterEvent {
    BeforeNavigate { from: Option<Location>, to: Location },
    BeforeLoad { from: Option<Location>, to: Location },
    Load { from: Option<Location>, to: Location },
    BeforeRouteMount { from: Option<Location>, to: Location },
    Resolved { from: Option<Location>, to: Location },
    Rendered { from: Option<Location>, to: Location },
}

/// Trait for route definitions.
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
    pub query_client: QueryClient,
    loader_factories: HashMap<String, LoaderFactory>,
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
        window_id: WindowId,
        window_handle: AnyWindowHandle,
        route_tree: Rc<RouteTree>,
        options: RouterOptions,
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
            not_found_mode: options.not_found_mode,
            not_found_data: None,
            default_pending_ms: options.default_pending_ms,
            default_pending_min_ms: options.default_pending_min_ms,
        }
    }

    pub fn set_root_view(&mut self, view_id: EntityId) {
        self.root_view = Some(view_id);
    }

    fn commit_navigation(&mut self, loc: Location, options: NavigateOptions, cx: &mut App) {
        let prev_route_ids: HashSet<String> = self
            .current_match
            .as_ref()
            .map(|(_, node)| {
                self.route_tree
                    .ancestors(&node.id)
                    .iter()
                    .map(|n| n.id.clone())
                    .chain(std::iter::once(node.id.clone()))
                    .collect()
            })
            .unwrap_or_default();

        self.current_match = self
            .route_tree
            .match_path(&loc.pathname)
            .map(|(params, node)| (params, node.clone()));

        let new_route_ids: HashSet<String> = self
            .current_match
            .as_ref()
            .map(|(_, node)| {
                self.route_tree
                    .ancestors(&node.id)
                    .iter()
                    .map(|n| n.id.clone())
                    .chain(std::iter::once(node.id.clone()))
                    .collect()
            })
            .unwrap_or_default();

        for route_id in prev_route_ids.difference(&new_route_ids) {
            if let Some(node) = self.route_tree.get_node(route_id) {
                if let Some(on_leave) = &node.on_leave {
                    on_leave(&loc);
                }
            }
        }

        for route_id in new_route_ids.difference(&prev_route_ids) {
            if let Some(node) = self.route_tree.get_node(route_id) {
                if let Some(on_enter) = &node.on_enter {
                    on_enter(&loc);
                }
            }
        }

        if options.replace {
            self.history.replace(loc.clone());
        } else {
            self.history.push(loc.clone());
        }

        self.trigger_loader_with_locations(None, loc, cx);
    }

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

        push_event(RouterEvent::BeforeNavigate { from: from.clone(), to: to.clone() }, cx);

        let (params, matched_node) = match self.route_tree.match_path(&loc.pathname) {
            Some((params, node)) => (params, node.clone()),
            None => {
                self.commit_navigation(loc, options, cx);
                return;
            }
        };

        let before_load_fns: Vec<(String, crate::route_tree::BeforeLoadFn)> = self
            .route_tree
            .ancestors(&matched_node.id)
            .iter()
            .map(|&node| node)
            .chain(std::iter::once(&matched_node))
            .filter_map(|node: &RouteNode| {
                node.before_load
                    .as_ref()
                    .map(|f| (node.id.clone(), f.clone()))
            })
            .collect();

        if !before_load_fns.is_empty() {
            let window_handle = self.window_handle;
            let params = params.clone();
            let loc = loc.clone();
            let before_load_fns = before_load_fns.clone();

            cx.spawn(move |cx: &mut gpui::AsyncApp| {
                let cx = cx.clone();
                async move {
                    for (_route_id, before_load) in before_load_fns {
                        let ctx = BeforeLoadContext {
                            params: params.clone(),
                            search: loc.search.clone(),
                            location: loc.clone(),
                        };
                        match before_load(ctx).await {
                            BeforeLoadResult::Ok => continue,
                            BeforeLoadResult::Redirect(redirect) => {
                                let nav = crate::Navigator::new(window_handle);
                                let _ = cx.update(|cx| {
                                    nav.push_location(Location::new(&redirect.to), cx);
                                });
                                return;
                            }
                            BeforeLoadResult::NotFound(not_found) => {
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
                    let _ = cx.update(|cx| {
                        RouterState::update(cx, |state, cx| {
                            state.commit_navigation(loc, options, cx);
                        });
                    });
                }
            })
            .detach();
            return;
        }

        self.commit_navigation(loc, options, cx);
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
                let _cx = _cx.clone();
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
        log::debug!("Registering rs-query loader factory for route: {}", route_id);
        self.loader_factories.insert(route_id.to_string(), factory);
    }

    fn trigger_loader_with_locations(&mut self, _from: Option<Location>, to: Location, cx: &mut App) {
        if let Some((params, node)) = &self.current_match {
            if node.has_loader {
                let route_id = node.id.clone();
                if let Some(factory) = self.loader_factories.get(&route_id) {
                    let query = factory(params);
                    let key = query.key.clone();
                    let client = self.query_client.clone();
                    let fetch_fn = query.fetch_fn.clone();
                    let options = query.options.clone();

                    let window_handle = self.window_handle;
                    let to_clone = to.clone();
                    cx.spawn(move |cx: &mut gpui::AsyncApp| {
                        let cx = cx.clone();
                        async move {
                            match (fetch_fn)().await {
                                Ok(outcome) => match outcome {
                                    LoaderOutcome::Data(data) => {
                                        client.set_query_data(&key, data, options.clone());
                                        let _ = cx.update(|cx| {
                                            push_event(RouterEvent::Load { from: None, to: to_clone.clone() }, cx);
                                            push_event(RouterEvent::BeforeRouteMount { from: None, to: to_clone.clone() }, cx);
                                            push_event(RouterEvent::Rendered { from: None, to: to_clone }, cx);
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
                                        push_event(RouterEvent::Load { from: None, to: to_clone.clone() }, cx);
                                        push_event(RouterEvent::BeforeRouteMount { from: None, to: to_clone.clone() }, cx);
                                        push_event(RouterEvent::Rendered { from: None, to: to_clone }, cx);
                                        let _ = window_handle.update(cx, |_, window, _| window.refresh());
                                    });
                                }
                            }
                        }
                    })
                    .detach();
                } else {
                    log::warn!("No loader factory registered for route: {}", route_id);
                    self.proceed_without_loader(to, cx);
                }
            } else {
                self.proceed_without_loader(to, cx);
            }
        }
    }

    fn proceed_without_loader(&self, to: Location, cx: &mut App) {
        push_event(RouterEvent::Load { from: None, to: to.clone() }, cx);
        push_event(RouterEvent::BeforeRouteMount { from: None, to: to.clone() }, cx);
        push_event(RouterEvent::Rendered { from: None, to }, cx);
    }

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
        let key = QueryKey::new("navi_loader")
            .with("route", node.id.as_str())
            .with("params", serde_json::to_string(params).unwrap_or_default())
            .with("deps", serde_json::to_string(&deps_json).unwrap_or_default());

        if self.query_client.is_in_flight(&key) {
            LoaderState::Loading
        } else if self.query_client.get_query_data::<AnyData>(&key).is_some() {
            LoaderState::Ready
        } else {
            LoaderState::Idle
        }
    }

    pub fn get_route_context<R: crate::RouteDef>(&self) -> Option<serde_json::Value> {
        None // TODO
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
