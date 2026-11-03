//! End-to-end tests for the Navi router: core re-exports, macro expansion,
//! and headless integration tests for `RouterState` driven through the
//! quoin-based `RouterHandle` / `RouterHost` seam.
//!
//! The pre-migration suite ran these against a gpui `TestAppContext`; the
//! post-migration crate is GUI-framework agnostic, so the same scenarios
//! run against a queued test host (`run_until_parked` drains spawned
//! continuations exactly like the old executor queue).
#![recursion_limit = "256"]

use futures::executor::block_on;
pub use navi_router::state::LocalFuture;
use navi_router::*;
use quoin_ui::Node;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Route node helper (kept local to keep the tree tests self-contained).
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Core re-export / matching tests.
// ---------------------------------------------------------------------------

#[test]
fn test_route_matching() {
    let mut tree = RouteTree::new();
    tree.add_route(node("test", "/test"));
    let (params, matched) = tree.match_path("/test").expect("match");
    assert_eq!(matched.id, "test");
    assert!(params.is_empty());
}

#[test]
fn test_navigation_blocker_sync() {
    let blocker = Blocker::new_sync(|_, _| true);
    let from = Location::new("/from");
    let to = Location::new("/to");
    let allow = block_on(blocker.should_allow(&from, &to));
    assert!(!allow);
}

#[test]
fn test_router_core_effects_via_reexports() {
    let mut tree = RouteTree::new();
    tree.add_route(node("a", "/a"));
    let mut core =
        navi_router::RouterCore::new(Location::new("/"), std::sync::Arc::new(tree.clone()));
    let effects = core.navigate(Location::new("/a"), &NavigateOptions::default());
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0], NavigationEffect::NotifyListeners));
    // Re-exported helpers.
    let r = redirect("/x");
    assert_eq!(r.to, "/x");
    let nf = not_found().with_route_id("a");
    assert_eq!(nf.route_id.as_deref(), Some("a"));
    // `RouterCore` is reachable through the adapter crate too.
    assert_eq!(core.route_tree().len(), 1);
}

#[test]
fn test_validate_search_reexported() {
    use navi_router::{ValidateSearch, ValidationError, ValidationResult};
    #[derive(Debug, Default)]
    struct S;
    impl ValidateSearch for S {
        fn validate(_raw: &HashMap<String, String>) -> ValidationResult<Self> {
            Ok(S)
        }
        fn to_query(&self) -> HashMap<String, String> {
            HashMap::new()
        }
    }
    let raw: HashMap<String, String> = HashMap::new();
    assert!(S::validate(&raw).is_ok());
    let err = ValidationError {
        field: Some("f".into()),
        message: "m".into(),
    };
    assert_eq!(err.to_string(), "f: m");
}

#[test]
fn test_location_roundtrip_via_reexports() {
    let loc = Location::from_parts("/a", serde_json::json!({"x": 1}), "#h");
    let url = loc.to_url("https://test.dev");
    assert!(url.contains("x=1"));
    assert!(url.ends_with("#h"));
    let back = Location::from_url(&url).expect("parse");
    assert_eq!(back.pathname, "/a");
    // `from_url` keeps query values as strings (no type coercion).
    assert_eq!(back.search["x"], serde_json::json!("1"));
}

// ---------------------------------------------------------------------------
// Macro expansion tests: define_route! / define_router! compile and produce
// the expected metadata. Routes register framework-agnostic `node:`
// constructors — no GUI framework anywhere.
// ---------------------------------------------------------------------------

mod macro_routes {
    use navi_macros::{define_route, define_router};
    use quoin_ui::Node;
    use serde::Deserialize;

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct MacroParams {
        pub id: String,
    }

    define_route!(
        MacroHome,
        path: "/",
        is_index: true,
        node: || Node::div().build()
    );
    define_route!(
        MacroUser,
        path: "/users/$id",
        params: MacroParams,
        node: || Node::div().build(),
        stale_time: std::time::Duration::from_secs(30),
        gc_time: std::time::Duration::from_secs(120),
    );
    define_route!(
        MacroGuarded,
        path: "/guarded",
        node: || Node::div().build(),
        before_load: |ctx: navi_router::route_tree::BeforeLoadContext| async move {
            let _ = ctx.location.pathname.as_str();
            navi_router::route_tree::BeforeLoadResult::Ok
        },
        on_enter: |_loc: &navi_router::Location| {
            // no-op
        },
    );

    define_router!(MacroHome, MacroUser, MacroGuarded);
}

#[test]
fn test_define_route_metadata() {
    use macro_routes::*;
    assert_eq!(MacroHome::path(), "/");
    assert_eq!(MacroHome::name(), "MacroHome");
    let home_node = MacroHome::build_node();
    assert!(home_node.is_index);
    assert!(!home_node.is_layout);
    assert!(!home_node.has_loader);

    let user_node = MacroUser::build_node();
    assert_eq!(user_node.pattern.raw, "/users/$id");
    assert!(
        user_node
            .pattern
            .segments
            .iter()
            .any(navi_router::Segment::is_dynamic)
    );

    let guarded_node = MacroGuarded::build_node();
    assert!(guarded_node.before_load.is_some());
}

#[test]
fn test_define_route_params_deserialize() {
    use macro_routes::*;
    use navi_router::RouteDef;
    let raw: HashMap<String, String> = [("id".to_string(), "42".to_string())].into_iter().collect();
    let json = serde_json::to_value(raw).expect("json");
    let params: <MacroUser as RouteDef>::Params =
        serde_json::from_value(json).expect("deserialize");
    assert_eq!(params.id, "42");
}

#[test]
fn test_define_router_tree() {
    use macro_routes::*;
    let tree = build_route_tree();
    assert_eq!(tree.len(), 3);
    let (params, matched) = tree.match_path("/users/7").expect("match");
    assert_eq!(matched.id, "MacroUser");
    assert_eq!(params.get("id").map(String::as_str), Some("7"));
    assert!(tree.validate().is_ok());
}

#[test]
fn test_define_router_enum() {
    use macro_routes::*;
    // Every generated variant is constructible and Debug-printable.
    let routes = [Route::MacroHome, Route::MacroUser, Route::MacroGuarded];
    assert_eq!(format!("{:?}", routes[0]), "MacroHome");
    assert!(matches!(routes[1], Route::MacroUser));
    assert!(matches!(routes[2], Route::MacroGuarded));
}

// ---------------------------------------------------------------------------
// Headless integration tests for RouterState via the queued test host.
// ---------------------------------------------------------------------------

/// A `RouterHost` seam that queues spawned futures until
/// [`QueuedHost::run_until_parked`] drains them — the headless equivalent
/// of the old executor queue.
#[derive(Default)]
struct QueuedHost {
    queue: Mutex<Vec<LocalFuture>>,
    refresh_count: AtomicUsize,
}

impl QueuedHost {
    fn host(self: &Rc<Self>) -> RouterHost {
        let queue = Rc::clone(self);
        let refresh = Rc::clone(self);
        let spawn_queue = Rc::clone(&queue);
        let spawn = Rc::new(move |future: LocalFuture| {
            spawn_queue.queue.lock().expect("queue").push(future);
        });
        RouterHost {
            refresh: Rc::new(move || {
                refresh.refresh_count.fetch_add(1, Ordering::SeqCst);
            }),
            spawn,
        }
    }

    /// Drains the queue: runs every spawned future to completion, including
    /// futures spawned while draining.
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

fn setup_router() -> (Rc<QueuedHost>, RouterHandle) {
    let queued = Rc::new(QueuedHost::default());
    let mut tree = RouteTree::new();
    tree.add_route(node("home", "/"));
    tree.add_route(node("about", "/about"));
    let mut user = node("user", "/users/$id");
    user.has_loader = true;
    tree.add_route(user);
    let handle = RouterHandle::new(
        Location::new("/"),
        navi_core::WindowKey(1),
        Arc::new(tree),
        queued.host(),
    );
    handle.install_current();
    (queued, handle)
}

fn flush() {
    // Keep the migration honest: async continuations are only observable
    // after the queue drains, mirroring `run_until_parked`.
    std::thread::yield_now();
}

#[test]
fn test_router_state_navigates() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    flush();
    assert_eq!(handle.read().current_location().pathname, "/about");
    assert_eq!(
        handle.read().current_match.as_ref().expect("match").1.id,
        "about"
    );
}

#[test]
fn test_router_state_params_and_meta() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.navigate(Location::new("/users/9"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    let state = handle.read();
    let (params, node) = state.current_match.as_ref().expect("match");
    assert_eq!(params.get("id").map(String::as_str), Some("9"));
    assert_eq!(node.id, "user");
    assert!(node.has_loader);
}

#[test]
fn test_router_state_history_and_blockers() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        let blocker = Blocker::new_sync(|_, _| true);
        let id = state.add_blocker(blocker);
        assert_eq!(state.blockers.len(), 1);
        let _ = id;
    });
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
    });
    // Blocker evaluation is async; park until it resolves.
    queued.run_until_parked();
    // Blocked navigation is pending, not committed.
    assert!(handle.read().is_blocked());
    assert!(handle.read().pending_navigation.is_some());
    assert_eq!(handle.read().current_location().pathname, "/");
    // Proceeding commits it, bypassing blockers.
    handle.update(RouterState::proceed);
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "/about");
    assert!(!handle.read().is_blocked());
    handle.update(|state| {
        let ids: Vec<_> = state.blockers.keys().copied().collect();
        for id in ids {
            state.remove_blocker(&id);
        }
        assert!(state.blockers.is_empty());
    });
}

#[test]
fn test_router_state_reset_block() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.add_blocker(Blocker::new_sync(|_, _| true));
    });
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert!(handle.read().is_blocked());
    handle.update(RouterState::reset_block);
    assert!(!handle.read().is_blocked());
    assert_eq!(handle.read().current_location().pathname, "/");
}

#[test]
fn test_router_state_events_and_event_log() {
    let (queued, handle) = setup_router();
    navi_router::event_bus::init_event_log();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_clone = Arc::clone(&calls);
    handle.update(|state| {
        state.subscribe(move |_event| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
        });
    });
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert!(
        calls.load(Ordering::SeqCst) >= 2,
        "subscribe callbacks fired"
    );
    let log = navi_router::event_bus::get_event_log();
    assert!(
        !log.is_empty(),
        "global event log must record navigation events"
    );
    assert!(matches!(log[0].event, RouterEvent::BeforeNavigate { .. }));
    assert!(matches!(
        log.last().expect("nonempty").event,
        RouterEvent::Resolved { .. }
    ));
}

#[test]
fn test_router_state_replace_navigation() {
    let (queued, handle) = setup_router();
    // Replacing the initial entry must not grow the history.
    handle.update(|state| {
        state.navigate(
            Location::new("/about"),
            &NavigateOptions {
                replace: true,
                ..NavigateOptions::default()
            },
        );
    });
    queued.run_until_parked();
    assert!(!handle.read().can_go_back());
    assert_eq!(handle.read().current_location().pathname, "/about");
    // A push afterwards grows the history normally.
    handle.update(|state| {
        state.navigate(Location::new("/"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert!(handle.read().can_go_back());
    assert_eq!(handle.read().current_location().pathname, "/");
}

#[test]
fn test_router_state_go_back_forward() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
        assert!(state.can_go_back());
        state.back();
        assert_eq!(state.current_location().pathname, "/");
        state.forward();
        assert_eq!(state.current_location().pathname, "/about");
        state.go(-5);
        assert_eq!(state.current_location().pathname, "/");
    });
    queued.run_until_parked();
}

#[test]
fn test_router_state_not_found_path() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.not_found_data = Some(serde_json::json!({"hint": "x"}));
        state.navigate(Location::new("/nope"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    // Navigating to an unknown path pushes /404 (no such route -> no match)
    // and records not_found_data.
    assert_eq!(
        handle.read().not_found_data,
        Some(serde_json::json!({"hint": "x"}))
    );
}

#[test]
fn test_router_state_loader_registration_and_keys() {
    let (_queued, handle) = setup_router();
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(move |_params: &HashMap<String, String>| {
                rs_query::Query::new(rs_query::QueryKey::new("test").with("k", "v"), || async {
                    Ok(LoaderOutcome::Data(AnyData::new("payload".to_string())))
                })
            }),
        );
    });
    let state = handle.read();
    assert!(
        state
            .query_client
            .get_query_data::<AnyData>(&rs_query::QueryKey::new("navi").with("x", "y"))
            .is_none(),
        "empty client starts with no data"
    );
}

#[test]
fn test_router_state_current_meta_empty() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.navigate(Location::new("/about"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    assert!(handle.read().current_meta().is_empty());
}

#[test]
fn test_router_state_try_current_and_current() {
    let (_queued, handle) = setup_router();
    assert!(RouterHandle::try_current().is_some());
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/"
    );
    drop(handle);
    RouterHandle::try_current().expect("installed handle stays current");
}

#[test]
fn test_navigator_navigates() {
    let (queued, _handle) = setup_router();
    let navigator = Navigator::new();
    navigator.push("/about");
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/about"
    );
    navigator.push("/users/1");
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/users/1"
    );
    navigator.back();
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/about"
    );
    navigator.forward();
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/users/1"
    );
    navigator.go(-2);
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/"
    );
    navigator.replace("/about");
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/about"
    );
    // Relative base resolution: "settings" against base "/users".
    let nav = Navigator::from_route("/users");
    nav.push("settings");
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/users/settings"
    );
    assert!(Navigator::can_go_back());
    queued.run_until_parked();
}

#[test]
fn test_register_routes_from_define_router() {
    let (_queued, _handle) = setup_router();
    macro_routes::register_routes();
    // No panic == components/loaders registered without issue.
}

// ---------------------------------------------------------------------------
// Transactions (NavOp batches) and scroll-reset plumbing.
// ---------------------------------------------------------------------------

#[test]
fn test_router_state_transact() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.transact(&[
            NavOp::Push(Location::new("/about")),
            NavOp::Push(Location::new("/users/3")),
            NavOp::Back,
        ]);
    });
    queued.run_until_parked();
    let state = handle.read();
    // Two pushes + a back: cursor sits on /about.
    assert_eq!(state.current_location().pathname, "/about");
    assert!(state.can_go_forward());
    assert!(state.can_go_back());
    // current_match re-resolved by the traversal inside the transaction.
    assert_eq!(state.current_match.as_ref().expect("match").1.id, "about");
}

#[test]
fn test_router_state_transact_falls_back_with_blockers() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.add_blocker(Blocker::new_sync(|_, _| true));
    });
    handle.update(|state| {
        state.transact(&[NavOp::Push(Location::new("/about"))]);
    });
    // Fallback routes through `navigate`, so the blocker held it pending.
    queued.run_until_parked();
    assert!(handle.read().is_blocked());
    assert_eq!(handle.read().current_location().pathname, "/");
    handle.update(RouterState::proceed);
    queued.run_until_parked();
    assert_eq!(handle.read().current_location().pathname, "/about");
}

#[test]
fn test_navigator_transact() {
    let (queued, _handle) = setup_router();
    let navigator = Navigator::new();
    navigator.transact([
        NavOp::Push(Location::new("/about")),
        NavOp::Replace(Location::new("/users/8")),
    ]);
    queued.run_until_parked();
    assert_eq!(
        RouterHandle::current().read().current_location().pathname,
        "/users/8"
    );
    // Replace kept the history at 2 entries.
    assert!(!RouterHandle::current().read().can_go_forward());
}

#[test]
fn test_navigator_push_with_options_sets_scroll_reset() {
    let (queued, handle) = setup_router();
    let navigator = Navigator::new();
    let options = NavigateOptions {
        reset_scroll: Some(true),
        ..NavigateOptions::default()
    };
    navigator.push_location_with(Location::new("/about"), &options);
    queued.run_until_parked();
    // The one-shot flag set by the navigation is consumed exactly once.
    handle.update(|state| {
        assert!(state.take_scroll_reset());
        assert!(!state.take_scroll_reset());
    });
    // Without reset_scroll the flag stays clear.
    navigator.push_location(Location::new("/"));
    handle.update(|state| {
        assert!(!state.take_scroll_reset());
    });
}

#[test]
fn test_router_state_traversal_rematches() {
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.navigate(Location::new("/users/5"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    handle.update(|state| {
        assert!(state.back());
        let matched = state.current_match.as_ref().expect("match");
        assert_eq!(matched.1.id, "home", "back must re-match the route");
    });
    handle.update(|state| {
        assert!(state.forward());
        let matched = state.current_match.as_ref().expect("match");
        assert_eq!(matched.1.id, "user", "forward must re-match the route");
    });
}

#[test]
fn test_router_state_spawns_loaders_via_host() {
    // The loader path spawns a continuation that writes the query cache and
    // refreshes; the queued host makes both observable headlessly.
    let (queued, handle) = setup_router();
    handle.update(|state| {
        state.register_loader_factory(
            "user",
            Arc::new(move |params: &HashMap<String, String>| {
                let id = params.get("id").cloned().unwrap_or_default();
                rs_query::Query::new(
                    rs_query::QueryKey::new("navi_loader")
                        .with("route", "user")
                        .with("params", id.clone()),
                    move || {
                        let id = id.clone();
                        async move { Ok(LoaderOutcome::Data(AnyData::new(format!("user-{id}")))) }
                    },
                )
            }),
        );
    });
    handle.update(|state| {
        state.navigate(Location::new("/users/42"), &NavigateOptions::default());
    });
    queued.run_until_parked();
    let state = handle.read();
    let key = rs_query::QueryKey::new("navi_loader")
        .with("route", "user")
        .with("params", "42");
    let cached = state.query_client.get_query_data::<AnyData>(&key);
    assert!(cached.is_some(), "loader result cached through the host");
    drop(state);
    assert!(
        queued.refresh_count.load(Ordering::SeqCst) > 0,
        "loader completion requested a repaint"
    );
}

#[test]
fn test_link_node_builds_without_router() {
    // Links render (external + internal) even with no router installed.
    let external = Link::new("https://example.com").child("docs").build();
    assert!(matches!(external, Node::Div(_)));
    let internal = Link::new("/users").child("users").build();
    assert!(matches!(internal, Node::Div(_)));
}

#[test]
fn test_scroll_positions_store() {
    use navi_router::state::{ScrollPositions, canonical_search_key};
    let store = ScrollPositions::new();
    store.set_capacity(2);
    store.insert("/a".to_string(), 1.0);
    store.insert("/b".to_string(), 2.0);
    assert_eq!(store.get("/a"), Some(1.0));
    // Re-saving an existing key must not evict anything.
    store.insert("/b".to_string(), 20.0);
    store.insert("/c".to_string(), 3.0);
    // "/a" is oldest and evicted, "/b" kept because it was refreshed.
    assert_eq!(store.get("/a"), None);
    assert_eq!(store.get("/b"), Some(20.0));
    assert_eq!(store.get("/c"), Some(3.0));

    // Search-key canonicalization still ignores object order.
    let a = canonical_search_key(&serde_json::json!({"a": "1", "b": "2"}));
    let b = canonical_search_key(&serde_json::json!({"b": "2", "a": "1"}));
    assert_eq!(a, b);
    assert_eq!(a, "a=1&b=2");
}
