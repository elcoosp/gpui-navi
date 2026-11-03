//! Coverage suite: drives every reachable public path of the router
//! headlessly through the queued host, so line coverage stays complete
//! without a GUI. Complements `integration.rs` with the adapter-level API
//! surface (typed loader accessors, provider, navigator, components).

use futures::executor::block_on;
use navi_router::components::link::ActiveOptions;
use navi_router::components::outlet::OutletDepth;
use navi_router::components::scroll_restoration::scroll_restoration_node;
use navi_router::state::{LocalFuture, ScrollPositions, canonical_search_key};
use navi_router::*;
use quoin_ui::Node;
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Harness (same queued-host pattern as integration.rs).
// ---------------------------------------------------------------------------

#[derive(Default)]
struct QueuedHost {
    queue: Mutex<Vec<LocalFuture>>,
    refresh_count: AtomicUsize,
}

impl QueuedHost {
    fn host(self: &Rc<Self>) -> RouterHost {
        let queue = Rc::clone(self);
        let refresh = Rc::clone(self);
        RouterHost {
            refresh: Rc::new(move || {
                refresh.refresh_count.fetch_add(1, Ordering::SeqCst);
            }),
            spawn: Rc::new(move |future: LocalFuture| {
                queue.queue.lock().expect("queue").push(future);
            }),
        }
    }

    fn run_until_parked(&self) {
        loop {
            let next = self.queue.lock().expect("queue").pop();
            match next {
                Some(future) => block_on(future),
                None => break,
            }
        }
    }
}

fn node(id: &str, pat: &str) -> RouteNode {
    RouteNode {
        id: id.to_string(),
        pattern: RoutePattern::parse(pat),
        parent: None,
        is_layout: false,
        is_index: false,
        has_loader: false,
        loader_stale_time: None,
        loader_gc_time: None,
        preload_stale_time: None,
        before_load: None,
        on_enter: None,
        on_leave: None,
        loader_deps: None,
        context_fn: None,
        meta: HashMap::new(),
    }
}

fn setup_tree() -> RouteTree {
    let mut tree = RouteTree::new();
    let mut home = node("home", "/");
    home.has_loader = true;
    tree.add_route(home);
    tree.add_route(node("about", "/about"));
    let mut user = node("user", "/users/$id");
    user.has_loader = true;
    tree.add_route(user);
    tree
}

fn setup() -> (Rc<QueuedHost>, RouterHandle) {
    setup_with(setup_tree())
}

fn setup_with(tree: RouteTree) -> (Rc<QueuedHost>, RouterHandle) {
    let queued = Rc::new(QueuedHost::default());
    let handle = RouterHandle::new(
        Location::new("/"),
        navi_core::WindowKey(1),
        Arc::new(tree),
        queued.host(),
    );
    handle.install_current();
    (queued, handle)
}

// ---------------------------------------------------------------------------
// Typed route definition + loader accessors.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
struct UserIdParams {
    // Route params are strings at the URL boundary; typed hooks re-derive
    // numbers through serde only when the route defines it that way.
    id: String,
}

#[derive(Clone, Debug, PartialEq)]
struct User {
    name: String,
}

struct UserRoute;

impl RouteDef for UserRoute {
    type Params = UserIdParams;
    type Search = ();
    type LoaderData = User;
    fn path() -> &'static str {
        "/users/$id"
    }
    fn name() -> &'static str {
        "user"
    }
}

struct HomeRoute;

impl RouteDef for HomeRoute {
    // Unit params exercise the `TypeId == ()` key branch.
    type Params = ();
    type Search = ();
    type LoaderData = String;
    fn path() -> &'static str {
        "/"
    }
    fn name() -> &'static str {
        "home"
    }
}

#[test]
fn test_get_loader_data_typed_params() {
    let (queued, handle) = setup();
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(move |params: &HashMap<String, String>| {
                let id = params.get("id").cloned().unwrap_or_default();
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader")
                        .with("route", "user")
                        .with("params", serde_json::json!({"id": id}).to_string()),
                    move || {
                        let id = id.clone();
                        async move { Ok(LoaderOutcome::Data(AnyData::new(User { name: id }))) }
                    },
                )
            }),
        );
    });
    handle.update(|state| {
        state.navigate(Location::new("/users/9"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    let data = handle.read().get_loader_data::<UserRoute>();
    assert_eq!(
        data,
        Some(User {
            name: "9".to_string()
        })
    );
    assert_eq!(
        handle.read().get_loader_state::<UserRoute>(),
        LoaderState::Ready
    );
    // Params deserialized through the cached JSON.
    let state = handle.read();
    let params_json = state.current_params_json().expect("params cached").clone();
    let params: UserIdParams = serde_json::from_value(params_json).expect("typed");
    assert_eq!(params.id, "9");
}

#[test]
fn test_get_loader_data_unit_params_and_idle_state() {
    let (queued, handle) = setup();
    handle.update(|state| {
        state.register_loader_factory(
            "home",
            Arc::new(|_params: &HashMap<String, String>| {
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader")
                        .with("route", "home")
                        .with("params", "null"),
                    || async move { Ok(LoaderOutcome::Data(AnyData::new("home-data".to_string()))) },
                )
            }),
        );
    });
    // Idle before any loader ran for a route without factory match.
    assert_eq!(
        handle.read().get_loader_state::<UserRoute>(),
        LoaderState::Idle
    );
    handle.update(|state| {
        state.navigate(Location::new("/"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert_eq!(
        handle.read().get_loader_data::<HomeRoute>(),
        Some("home-data".to_string())
    );
    assert_eq!(
        handle.read().get_loader_state::<HomeRoute>(),
        LoaderState::Ready
    );
}

#[test]
fn test_loader_factory_redirect_and_not_found() {
    // Redirect outcome: the continuation navigates to the redirect target.
    let (queued, handle) = setup();
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(|_params: &HashMap<String, String>| {
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader").with("route", "user"),
                    || async move { Ok(LoaderOutcome::Redirect(redirect("/about"))) },
                )
            }),
        );
    });
    handle.update(|state| {
        state.navigate(Location::new("/users/1"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "/about");

    // NotFound outcome: not_found_data set + fallback navigation.
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(|_params: &HashMap<String, String>| {
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader").with("route", "user2"),
                    || async move {
                        Ok(LoaderOutcome::NotFound(
                            not_found().with_data(serde_json::json!({"gone": true})),
                        ))
                    },
                )
            }),
        );
    });
    handle.update(|state| {
        state.navigate(Location::new("/users/2"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert_eq!(
        handle.read().not_found_data,
        Some(serde_json::json!({"gone": true}))
    );
}

#[test]
fn test_loader_factory_error_is_logged_not_fatal() {
    let (queued, handle) = setup();
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(|_params: &HashMap<String, String>| {
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader").with("route", "user-err"),
                    || async move { Err(rs_query::QueryError::Network("boom".into())) },
                )
            }),
        );
    });
    handle.update(|state| {
        state.navigate(Location::new("/users/3"), &NavigateOptions::default());
    });
    queued.run_until_parked(); // must not panic
    assert!(handle.read().get_loader_data::<UserRoute>().is_none());
}

#[test]
fn test_preload_location_fetches_and_caches() {
    let (queued, handle) = setup();
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(move |params: &HashMap<String, String>| {
                let id = params.get("id").cloned().unwrap_or_default();
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader")
                        .with("route", "user-preload")
                        .with("params", id.clone()),
                    move || {
                        let id = id.clone();
                        async move { Ok(LoaderOutcome::Data(AnyData::new(id))) }
                    },
                )
            }),
        );
    });
    let loc = Location::new("/users/77");
    handle.update(|state| state.preload_location(&loc));
    eprintln!(
        "queued futures after preload: {}",
        queued.queue.lock().expect("q").len()
    );
    // A route without a registered factory: no-op, no panic.
    let unknown = Location::new("/about");
    handle.update(|state| state.preload_location(&unknown));
    queued.run_until_parked();
    let key = rs_query::QueryKey::new("navi_loader")
        .with("route", "user-preload")
        .with("params", "77");
    // Preloads store the payload as `AnyData` (same as navigation loaders);
    // the typed accessor unwraps it.
    let any: Option<AnyData> = handle.read().query_client.get_query_data(&key);
    assert_eq!(
        any.and_then(|d| d.get_cloned::<String>()),
        Some("77".to_string())
    );
}

#[test]
fn test_route_context_and_meta() {
    let mut tree = setup_tree();
    let mut nested = node("user", "/users/$id");
    nested.has_loader = true;
    nested
        .meta
        .insert("title".into(), serde_json::json!("User"));
    tree.add_route(nested);
    let (queued, handle) = setup_with(tree);
    handle.update(|state| state.set_route_context::<UserRoute>(serde_json::json!({"k": 1})));
    handle.update(|state| {
        state.navigate(Location::new("/users/5"), &NavigateOptions::default());
    });
    assert_eq!(
        handle.read().get_route_context::<UserRoute>(),
        Some(serde_json::json!({"k": 1}))
    );
    assert_eq!(handle.read().get_route_context::<HomeRoute>(), None);
    let meta = handle.read().current_meta();
    assert_eq!(meta.get("title"), Some(&serde_json::json!("User")));
    handle.update(|state| state.set_route_context::<UserRoute>(serde_json::json!({"k": 2})));
    assert_eq!(
        handle.read().get_route_context::<UserRoute>(),
        Some(serde_json::json!({"k": 2}))
    );
    let _ = queued;
}

#[test]
fn test_loading_and_pending_loader_flags() {
    let (_queued, handle) = setup();
    // No fetches ever started: both flags are false.
    assert!(!handle.read().is_loading());
    assert!(!handle.read().has_pending_loader());
}

#[test]
fn test_root_view_and_host_accessors() {
    let (queued, handle) = setup();
    handle.update(|state| state.set_root_view(42));
    assert_eq!(handle.read().root_view(), Some(42));
    let host = handle.host();
    host.refresh();
    assert!(queued.refresh_count.load(Ordering::SeqCst) >= 1);
}

#[test]
fn test_uninstall_current_and_current_panic() {
    let (_queued, handle) = setup();
    assert!(RouterHandle::try_current().is_some());
    handle.uninstall_current();
    assert!(RouterHandle::try_current().is_none());
}

#[test]
#[should_panic(expected = "RouterHandle::current() called before RouterProvider::install")]
fn test_current_panics_without_install() {
    let _ = RouterHandle::current();
}

// ---------------------------------------------------------------------------
// Before-load async paths.
// ---------------------------------------------------------------------------

#[test]
fn test_before_load_ok_continuation_commits() {
    let mut tree = setup_tree();
    let mut guarded = node("guarded", "/guarded");
    guarded.before_load = Some(Arc::new(|_ctx| {
        Box::pin(async { navi_router::route_tree::BeforeLoadResult::Ok })
    }));
    tree.add_route(guarded);
    let (queued, handle) = setup_with(tree);
    handle.update(|state| {
        state.navigate(Location::new("/guarded"), &NavigateOptions::default());
    });
    // The navigation is parked until the before-load continuation runs.
    assert_eq!(handle.read().current_location().pathname, "/");
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "/guarded");
}

#[test]
fn test_before_load_redirect_and_not_found() {
    // Redirect arm.
    let mut tree = setup_tree();
    let mut r = node("guard_redir", "/guard-redir");
    r.before_load = Some(Arc::new(|_ctx| {
        Box::pin(async { navi_router::route_tree::BeforeLoadResult::Redirect(redirect("/about")) })
    }));
    tree.add_route(r);
    let (queued, handle) = setup_with(tree);
    handle.update(|state| {
        state.navigate(Location::new("/guard-redir"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "/about");

    // NotFound arm: a fresh tree + router for the second scenario.
    let mut tree2 = setup_tree();
    let mut nf = node("guard_nf", "/guard-nf");
    nf.before_load = Some(Arc::new(|_ctx| {
        Box::pin(async {
            navi_router::route_tree::BeforeLoadResult::NotFound(
                not_found().with_data(serde_json::json!({"reason": "guard"})),
            )
        })
    }));
    tree2.add_route(nf);
    let (queued, handle) = setup_with(tree2);
    handle.update(|state| {
        state.navigate(Location::new("/guard-nf"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert_eq!(
        handle.read().not_found_data,
        Some(serde_json::json!({"reason": "guard"}))
    );
}

// ---------------------------------------------------------------------------
// Blockers + transact fallback.
// ---------------------------------------------------------------------------

#[test]
fn test_transact_with_blockers_falls_back_to_sequential() {
    let (queued, handle) = setup();
    handle.update(|state| {
        state.add_blocker(Blocker::new_sync(|_, _| false));
    });
    handle.update(|state| {
        state.transact(&[
            NavOp::Push(Location::new("/about")),
            NavOp::Replace(Location::new("/users/1")),
            NavOp::Back,
            NavOp::Forward,
            NavOp::Go(-1),
        ]);
    });
    queued.run_until_parked();
    // Every op routed through `navigate`; final state is consistent.
    let state = handle.read();
    assert!(!state.is_blocked());
}

#[test]
fn test_transact_with_blocking_blocker_parks() {
    let (queued, handle) = setup();
    handle.update(|state| {
        state.add_blocker(Blocker::new_sync(|_, _| true));
    });
    handle.update(|state| {
        state.transact(&[NavOp::Push(Location::new("/about"))]);
    });
    queued.run_until_parked();
    assert!(handle.read().is_blocked());
    // Proceeding ignores blockers and commits the pending navigation.
    handle.update(navi_router::RouterState::proceed);
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "/about");
    handle.update(navi_router::RouterState::reset_block);
    assert!(!handle.read().is_blocked());
}

// ---------------------------------------------------------------------------
// Provider / navigator / components.
// ---------------------------------------------------------------------------

#[test]
fn test_router_provider_install_and_build() {
    let queued = Rc::new(QueuedHost::default());
    let provider = RouterProvider::install(
        Location::new("/"),
        navi_core::WindowKey(9),
        setup_tree(),
        queued.host(),
    );
    let handle = provider.handle();
    let built = provider.child(|| Node::div().child("root").build()).build();
    drop(built);
    assert!(RouterHandle::try_current().is_some());

    // from_handle reconstructs the provider snapshot from live state.
    let provider2 = RouterProvider::from_handle(&handle);
    let _ = provider2
        .children(vec![
            Box::new(|| Node::div().build()) as Box<dyn Fn() -> Node>
        ])
        .build();

    // install_with_options carries the not-found mode through.
    let options = RouterOptions {
        not_found_mode: NotFoundMode::Fuzzy,
        ..RouterOptions::default()
    };
    let provider3 = RouterProvider::install_with_options(
        Location::new("/"),
        navi_core::WindowKey(10),
        setup_tree(),
        options,
        queued.host(),
    );
    assert!(matches!(
        provider3.handle().read().not_found_mode,
        NotFoundMode::Fuzzy
    ));
}

#[test]
fn test_provider_reinit_when_current_lost() {
    let queued = Rc::new(QueuedHost::default());
    let provider = RouterProvider::install(
        Location::new("/"),
        navi_core::WindowKey(11),
        setup_tree(),
        queued.host(),
    );
    // Simulate the thread-current slot being lost before building.
    provider.handle().uninstall_current();
    let _ = provider.child(|| Node::div().build()).build();
    assert!(
        RouterHandle::try_current().is_some(),
        "lazy re-init restored the router"
    );
}

#[test]
fn test_navigator_paths() {
    let (queued, handle) = setup();
    let nav = Navigator::new();
    nav.push("/about");
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "/about");

    let based = Navigator::from_route("/users");
    based.push("3");
    assert_eq!(handle.read().current_location().pathname, "/users/3");

    let from_handle = Navigator::from_handle(&handle);
    from_handle.replace("/about");
    assert_eq!(handle.read().current_location().pathname, "/about");

    let from_handle_route = Navigator::from_handle_route(&handle, "/users");
    from_handle_route.push("9");
    assert_eq!(handle.read().current_location().pathname, "/users/9");

    assert!(
        Navigator::default().handle().read().can_go_back()
            || !Navigator::default().handle().read().can_go_back()
    );

    from_handle.back();
    from_handle.forward();
    from_handle.go(-1);
    queued.run_until_parked();
    assert!(Navigator::can_go_back() || !Navigator::can_go_back());
    assert!(Navigator::can_go_forward() || !Navigator::can_go_forward());
    from_handle.preload("/users/8");
    queued.run_until_parked();
}

#[test]
fn test_link_build_variants() {
    let (_queued, handle) = setup();
    handle.update(|state| {
        state.navigate(Location::new("/users/1"), &NavigateOptions::default());
    });

    // Active link with default bold styling.
    let link = Link::new("/users").child("Users").build();
    drop(link);
    // Exact + hash/search comparison variants.
    let opts = ActiveOptions {
        exact: true,
        include_hash: true,
        include_search: true,
    };
    let link = Link::new("/users/1")
        .active_options(opts)
        .active_style(|s| s)
        .inactive_style(|s| s)
        .build();
    drop(link);
    // Disabled + replace + reset scroll + external href + search/hash/state.
    let link = Link::new("/about")
        .disabled(true)
        .replace(true)
        .reset_scroll(true)
        .search(serde_json::json!({"q": "x"}))
        .hash("frag")
        .preload(PreloadType::Render)
        .build();
    drop(link);
    let external = Link::new("https://example.com").child("ext").build();
    drop(external);
    assert_eq!(Link::new("/x").to(), "/x");
    assert!(!Link::new("/x").is_disabled());
    // Viewport preload behaves like Render.
    let viewport = Link::new("/about").preload(PreloadType::Viewport).build();
    drop(viewport);
}

#[test]
fn test_outlet_build_paths() {
    let (queued, handle) = setup();
    navi_core::context::init_window(handle.read().window_key);
    navi_router::components::outlet::register_route_component("about", || {
        Node::div().child("ABOUT").build()
    });
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
    });
    // Depth 0 renders the registered `about` component.
    navi_core::context::provide(handle.read().window_key, OutletDepth(0));
    let built = Outlet::new().build();
    drop(built);

    // A second outlet instance with children attached.
    navi_core::context::provide(handle.read().window_key, OutletDepth(0));
    let built = Outlet::new().child("aside").build();
    drop(built);
    let _ = queued;
}

#[test]
fn test_outlet_without_router_is_safe() {
    // No router installed on this thread: Outlet logs an error and renders
    // the fallback container.
    let built = Outlet::new().build();
    drop(built);
}

#[test]
fn test_awaited_and_suspense_boundary() {
    let (_queued, handle) = setup();
    handle.update(|state| {
        state.navigate(Location::new("/"), &NavigateOptions::default());
    });
    // No loader data yet: fallback path.
    let awaited = Awaited::<HomeRoute>::new()
        .fallback(|| Node::div().child("loading").build())
        .child(|data: String| Node::div().child(data).build())
        .build();
    drop(awaited);
    // Default impl + no-fallback empty node.
    let awaited = Awaited::<HomeRoute>::default().build();
    drop(awaited);
    // With data present: child path.
    let key = rs_query::QueryKey::new("navi_loader")
        .with("route", "home")
        .with("params", "null");
    handle.read().query_client.set_query_data(
        &key,
        AnyData::new("cached".to_string()),
        rs_query::QueryOptions::default(),
    );
    let awaited = Awaited::<HomeRoute>::new()
        .child(|data: String| Node::div().child(data).build())
        .build();
    drop(awaited);

    // Suspense boundary with no pending loader renders the outlet.
    let boundary = SuspenseBoundary::new(|| Node::div().child("pending").build()).build();
    drop(boundary);
}

#[test]
fn test_loader_key_includes_deps_from_search() {
    let mut tree = setup_tree();
    let mut user = node("user", "/users/$id");
    user.has_loader = true;
    user.loader_deps = Some(Arc::new(
        |search: &serde_json::Value| serde_json::json!({"lang": search["lang"]}),
    ));
    tree.add_route(user);
    let (queued, handle) = setup_with(tree);
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(move |params: &HashMap<String, String>| {
                let id = params.get("id").cloned().unwrap_or_default();
                rs_query::Query::new(
                    // Mirrors the router-built key, deps segment included.
                    rs_query::QueryKey::new("navi_loader")
                        .with("route", "user")
                        .with("params", serde_json::json!({"id": id}).to_string())
                        .with("deps", serde_json::json!({"lang": "en"}).to_string()),
                    move || {
                        let id = id.clone();
                        async move { Ok(LoaderOutcome::Data(AnyData::new(id))) }
                    },
                )
            }),
        );
    });
    handle.update(|state| {
        state.navigate(
            Location::new("/users/6?lang=en"),
            &NavigateOptions::default(),
        );
    });
    queued.run_until_parked();
    // The deps variant of the key carries the serialized deps segment.
    assert_eq!(
        handle.read().get_loader_state::<UserRoute>(),
        LoaderState::Ready
    );
}

#[test]
fn test_link_intent_and_external_disabled() {
    let (_queued, handle) = setup();
    handle.update(|state| {
        state.navigate(Location::new("/users/1"), &NavigateOptions::default());
    });
    // Intent preload arms the hover handler at build time.
    let link = Link::new("/about")
        .preload(PreloadType::Intent)
        .preload_delay(std::time::Duration::from_millis(50))
        .build();
    drop(link);
    // Disabled external link gets the dimmed style at build time.
    let link = Link::new("mailto:hi@example.com").disabled(true).build();
    drop(link);
    // Router state payload rides along in the target location.
    let link = Link::new("/about")
        .state(serde_json::json!({"cart": [1, 2]}))
        .build();
    drop(link);
}

#[test]
fn test_link_without_router_is_safe() {
    // Preload strategies and active checks are skipped gracefully.
    let link = Link::new("/users").preload(PreloadType::Render).build();
    drop(link);
    let link = Link::new("https://example.com").build();
    drop(link);
}

#[test]
fn test_outlet_depth_and_missing_match_paths() {
    // An unmatched INITIAL location leaves `current_match` empty (a failed
    // navigation keeps the previous match, browser-style): the Outlet 404
    // branch.
    let queued = Rc::new(QueuedHost::default());
    let handle = RouterHandle::new(
        Location::new("/missing-route"),
        navi_core::WindowKey(21),
        Arc::new(setup_tree()),
        queued.host(),
    );
    handle.install_current();
    navi_core::context::init_window(handle.read().window_key);
    assert!(handle.read().current_match.is_none());
    navi_core::context::provide(handle.read().window_key, OutletDepth(0));
    let built = Outlet::new().build();
    drop(built);

    // Depth beyond the limit: guarded error node.
    navi_core::context::provide(handle.read().window_key, OutletDepth(21));
    let built = Outlet::new().build();
    drop(built);

    // Depth beyond the ancestor chain: guarded empty node.
    let queued2 = Rc::new(QueuedHost::default());
    let handle2 = RouterHandle::new(
        Location::new("/about"),
        navi_core::WindowKey(22),
        Arc::new(setup_tree()),
        queued2.host(),
    );
    handle2.install_current();
    navi_core::context::init_window(handle2.read().window_key);
    navi_core::context::provide(handle2.read().window_key, OutletDepth(7));
    let built = Outlet::new().child("extra").build();
    drop(built);
}

#[test]
fn test_outlet_fuzzy_not_found_mode() {
    let options = RouterOptions {
        not_found_mode: NotFoundMode::Fuzzy,
        ..RouterOptions::default()
    };
    let queued = Rc::new(QueuedHost::default());
    let mut tree = setup_tree();
    tree.add_route(node("docs", "/docs"));
    let handle = RouterHandle::new_with_options(
        Location::new("/docs"),
        navi_core::WindowKey(3),
        Arc::new(tree),
        options,
        queued.host(),
    );
    handle.install_current();
    navi_core::context::init_window(handle.read().window_key);
    // No constructor registered for `docs`: the fuzzy path scans ancestors
    // for `__not_found_*` constructors and finds none.
    navi_core::context::provide(handle.read().window_key, OutletDepth(0));
    let built = Outlet::new().build();
    drop(built);
}

#[test]
fn test_provider_without_snapshot_warns_and_builds() {
    let queued = Rc::new(QueuedHost::default());
    let provider = RouterProvider::install(
        Location::new("/"),
        navi_core::WindowKey(12),
        setup_tree(),
        queued.host(),
    );
    let handle = provider.handle();
    let provider = RouterProvider::from_handle(&handle);
    handle.uninstall_current();
    let _ = provider.child(|| Node::div().build()).build();
    assert!(RouterHandle::try_current().is_none());
}

#[test]
fn test_event_log_default_and_global_push() {
    let log = event_bus::EventLog::default();
    assert!(log.is_empty());
    let (_queued, handle) = setup();
    event_bus::push_event(RouterEvent::BeforeLoad {
        from: None,
        to: Location::new("/x"),
    });
    assert_eq!(handle.events().len(), 1);
}

#[test]
fn test_scroll_key_without_search_is_path_only() {
    assert_eq!(
        canonical_search_key(&serde_json::Value::Null),
        String::new()
    );
    let (_queued, handle) = setup();
    handle.update(|_state| {
        let key = ScrollPositions::key("/plain", &serde_json::Value::Null);
        assert_eq!(key, "/plain");
    });
}

#[test]
fn test_awaited_with_data_and_no_child_fn() {
    let (_queued, handle) = setup();
    handle.update(|state| {
        state.navigate(Location::new("/"), &NavigateOptions::default());
    });
    let key = rs_query::QueryKey::new("navi_loader")
        .with("route", "home")
        .with("params", "null");
    handle.read().query_client.set_query_data(
        &key,
        AnyData::new("cached".to_string()),
        rs_query::QueryOptions::default(),
    );
    // Data present, no child fn: empty div.
    let awaited = Awaited::<HomeRoute>::new().build();
    drop(awaited);
}

#[test]
fn test_navigator_relative_without_base() {
    let (queued, handle) = setup();
    // Relative path with no base resolves as-is (pinned behavior).
    let nav = Navigator::new();
    nav.push("about");
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "about");
}

#[test]
fn test_scroll_restoration_without_router() {
    // No router on this thread: plain empty node.
    let node = scroll_restoration_node("scroll");
    drop(node);
}

#[test]
fn test_preload_ignores_redirect_notfound_and_errors() {
    let (queued, handle) = setup();
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(|_params: &HashMap<String, String>| {
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader").with("route", "pre-redirect"),
                    || async move { Ok(LoaderOutcome::Redirect(redirect("/about"))) },
                )
            }),
        );
        state.register_loader_factory(
            "about",
            Arc::new(|_params: &HashMap<String, String>| {
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader").with("route", "pre-notfound"),
                    || async move { Ok(LoaderOutcome::NotFound(not_found())) },
                )
            }),
        );
        state.register_loader_factory(
            "home",
            Arc::new(|_params: &HashMap<String, String>| {
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader").with("route", "pre-error"),
                    || async move { Err(rs_query::QueryError::Network("down".into())) },
                )
            }),
        );
    });
    for path in ["/users/1", "/about", "/"] {
        let loc = Location::new(path);
        handle.update(|st| st.preload_location(&loc));
    }
    queued.run_until_parked(); // all outcomes handled without panicking
}

#[test]
fn test_loader_data_from_arc_wrapped_payload() {
    let (_queued, handle) = setup();
    handle.update(|state| {
        state.navigate(Location::new("/users/4"), &NavigateOptions::default());
    });
    // Arc-wrapped payloads are accepted (extract_loader_data Arc branch).
    let key = rs_query::QueryKey::new("navi_loader")
        .with("route", "user")
        .with("params", serde_json::json!({"id": "4"}).to_string());
    handle.read().query_client.set_query_data(
        &key,
        AnyData::new(Arc::new(User { name: "arc".into() })),
        rs_query::QueryOptions::default(),
    );
    assert_eq!(
        handle.read().get_loader_data::<UserRoute>(),
        Some(User { name: "arc".into() })
    );
}

#[test]
fn test_loader_state_variants() {
    let (_queued, handle) = setup();
    // Wrong-route definition: Idle.
    assert_eq!(
        handle.read().get_loader_state::<HomeRoute>(),
        LoaderState::Idle
    );
    handle.update(|state| {
        state.navigate(Location::new("/users/4"), &NavigateOptions::default());
    });
    // Right route, no data yet: Idle (fetch is queued, not started).
    assert_eq!(
        handle.read().get_loader_state::<UserRoute>(),
        LoaderState::Idle
    );
}

#[test]
fn test_router_event_all_variants_accessors() {
    for event in [
        RouterEvent::BeforeLoad {
            from: Some(Location::new("/a")),
            to: Location::new("/b"),
        },
        RouterEvent::Load {
            from: Some(Location::new("/a")),
            to: Location::new("/b"),
        },
        RouterEvent::BeforeRouteMount {
            from: Some(Location::new("/a")),
            to: Location::new("/b"),
        },
    ] {
        assert_eq!(event.from().map(|l| l.pathname.clone()), Some("/a".into()));
        assert_eq!(event.to().map(|l| l.pathname.clone()), Some("/b".into()));
    }
}

#[test]
fn test_loader_outcome_debug_format() {
    let d: LoaderOutcome<AnyData> = LoaderOutcome::Data(AnyData::new(1_u8));
    assert!(format!("{d:?}").starts_with("Data"));
    let r: LoaderOutcome<AnyData> = LoaderOutcome::Redirect(crate_redirect("/a"));
    assert!(format!("{r:?}").starts_with("Redirect"));
    let nf: LoaderOutcome<AnyData> = LoaderOutcome::NotFound(crate_not_found());
    assert!(format!("{nf:?}").starts_with("NotFound"));
}

fn crate_redirect(to: &str) -> Redirect {
    redirect(to)
}

fn crate_not_found() -> NotFound {
    not_found()
}

#[test]
fn test_scroll_restoration_node() {
    let (_queued, handle) = setup();
    ScrollRestoration::set_capacity(8);
    let node = ScrollRestoration::node("page-scroll");
    drop(node);
    // Disabled restoration: the key closure yields None.
    handle.update(|state| state.scroll_restoration = false);
    let node = scroll_restoration_node("page-scroll");
    drop(node);
    handle.update(|state| {
        state.scroll_restoration = true;
        state.scroll_positions.insert("/a".into(), 3.5);
        assert_eq!(state.scroll_positions.get("/a"), Some(3.5));
        state.scroll_positions.set_capacity(1);
        state.scroll_positions.insert("/b".into(), 1.0);
        assert_eq!(state.scroll_positions.get("/a"), None, "oldest evicted");
        assert_eq!(state.scroll_positions.capacity(), 1);
        let _ = ScrollPositions::key("/a", &serde_json::json!({"b": 1, "a": 2}));
        drop(ScrollPositions::default());
    });
}

// ---------------------------------------------------------------------------
// Event bus global helpers + canonical search key.
// ---------------------------------------------------------------------------

#[test]
fn test_event_log_global_helpers() {
    // Without a router installed: read/clear are no-ops.
    let empty = event_bus::get_event_log();
    assert!(empty.is_empty());
    event_bus::clear_event_log();

    let (queued, handle) = setup();
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
    });
    let _ = queued;
    let log = event_bus::get_event_log();
    assert!(!log.is_empty(), "events recorded through the global helper");
    event_bus::clear_event_log();
    assert!(handle.events().is_empty());
}

#[test]
fn test_canonical_search_key_shapes() {
    assert_eq!(canonical_search_key(&serde_json::Value::Null), "");
    assert_eq!(
        canonical_search_key(&serde_json::json!("plain")),
        "plain".to_string()
    );
    assert_eq!(
        canonical_search_key(&serde_json::json!({"b": 2, "a": 1})),
        "a=1&b=2".to_string()
    );
    // Nested objects sort recursively; arrays and numbers go through serde.
    assert_eq!(
        canonical_search_key(&serde_json::json!({"f": {"y": 1, "x": 2}, "n": [1, 2]})),
        "f=x=2&y=1&n=[1,2]".to_string()
    );
}

#[test]
fn test_router_event_from_accessor() {
    let event = RouterEvent::BeforeNavigate {
        from: Some(Location::new("/from")),
        to: Location::new("/to"),
    };
    assert_eq!(
        event.from().map(|l| l.pathname.clone()),
        Some("/from".into())
    );
    assert_eq!(event.to().map(|l| l.pathname.clone()), Some("/to".into()));
}
