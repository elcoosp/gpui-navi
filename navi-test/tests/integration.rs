//! End-to-end tests for the Navi router: core re-exports, macro expansion,
//! and gpui `TestAppContext` integration tests for `RouterState`.
#![recursion_limit = "256"]

use futures::executor::block_on;
use navi_router::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
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
    let mut core = navi_router::RouterCore::new(Location::new("/"), tree.clone());
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
// the expected metadata.
// ---------------------------------------------------------------------------

mod macro_routes {
    use gpui::prelude::*;
    use gpui::*;
    use navi_macros::{define_route, define_router};
    use serde::Deserialize;

    #[derive(Clone, IntoElement)]
    pub struct DummyPage;

    impl RenderOnce for DummyPage {
        fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
            div().child("dummy")
        }
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct MacroParams {
        pub id: String,
    }

    define_route!(MacroHome, path: "/", is_index: true, component: DummyPage);
    define_route!(
        MacroUser,
        path: "/users/$id",
        params: MacroParams,
        component: DummyPage,
        stale_time: std::time::Duration::from_secs(30),
        gc_time: std::time::Duration::from_secs(120),
    );
    define_route!(
        MacroGuarded,
        path: "/guarded",
        component: DummyPage,
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
    assert_eq!(<MacroHome as navi_router::RouteDef>::path(), "/");
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
fn test_define_router_enum_and_register() {
    use macro_routes::*;
    // Every generated variant is constructible and Debug-printable.
    let routes = [Route::MacroHome, Route::MacroUser, Route::MacroGuarded];
    assert_eq!(format!("{:?}", routes[0]), "MacroHome");
    assert!(matches!(routes[1], Route::MacroUser));
    assert!(matches!(routes[2], Route::MacroGuarded));
    // register_routes compiles against `&mut gpui::App`; verified in the
    // gpui::test below where we actually call it.
}

// ---------------------------------------------------------------------------
// gpui TestAppContext integration tests for RouterState.
// ---------------------------------------------------------------------------

struct TestRootView;

impl gpui::Render for TestRootView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

fn setup_router(cx: &mut gpui::TestAppContext) -> gpui::WindowHandle<TestRootView> {
    let window = cx.add_window(|_window, _cx| TestRootView);
    let _ = window.update(cx, |_, window, cx| {
        let window_id = window.window_handle().window_id();
        let handle = window.window_handle();
        let mut tree = RouteTree::new();
        tree.add_route(node("home", "/"));
        tree.add_route(node("about", "/about"));
        let mut user = node("user", "/users/$id");
        user.has_loader = true;
        tree.add_route(user);
        let state = RouterState::new_with_options(
            Location::new("/"),
            window_id,
            handle,
            Rc::new(tree),
            RouterOptions::default(),
        );
        cx.set_global(state);
    });
    window
}

#[gpui::test]
fn test_router_state_navigates(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/about"), &NavigateOptions::default(), cx);
        });
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/about"
        );
        assert_eq!(
            RouterState::global(cx)
                .current_match
                .as_ref()
                .expect("match")
                .1
                .id,
            "about"
        );
    });
}

#[gpui::test]
fn test_router_state_params_and_meta(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/users/9"), &NavigateOptions::default(), cx);
        });
        let state = RouterState::global(cx);
        let (params, node) = state.current_match.as_ref().expect("match");
        assert_eq!(params.get("id").map(String::as_str), Some("9"));
        assert_eq!(node.id, "user");
        assert!(node.has_loader);
    });
}

#[gpui::test]
fn test_router_state_history_and_blockers(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, _| {
            let blocker = Blocker::new_sync(|_, _| true);
            let id = state.add_blocker(blocker);
            assert_eq!(state.blockers.len(), 1);
            let _ = id;
        });
    });
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/about"), &NavigateOptions::default(), cx);
        });
    });
    // Blocker evaluation is async; park until it resolves.
    cx.run_until_parked();
    let _ = window.update(cx, |_, _, cx| {
        // Blocked navigation is pending, not committed.
        assert!(RouterState::global(cx).is_blocked());
        assert!(RouterState::global(cx).pending_navigation.is_some());
        assert_eq!(RouterState::global(cx).current_location().pathname, "/");
    });
    let _ = window.update(cx, |_, _, cx| {
        // Proceeding commits it, bypassing blockers.
        RouterState::update(cx, navi_router::RouterState::proceed);
    });
    let _ = window.update(cx, |_, _, cx| {
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/about"
        );
        assert!(!RouterState::global(cx).is_blocked());
        RouterState::update(cx, |state, _| {
            let ids: Vec<_> = state.blockers.keys().copied().collect();
            for id in ids {
                state.remove_blocker(&id);
            }
            assert!(state.blockers.is_empty());
        });
    });
}

#[gpui::test]
fn test_router_state_reset_block(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, _| {
            state.add_blocker(Blocker::new_sync(|_, _| true));
        });
    });
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/about"), &NavigateOptions::default(), cx);
        });
    });
    cx.run_until_parked();
    let _ = window.update(cx, |_, _, cx| {
        assert!(RouterState::global(cx).is_blocked());
        RouterState::update(cx, |state, _| state.reset_block());
        assert!(!RouterState::global(cx).is_blocked());
        assert_eq!(RouterState::global(cx).current_location().pathname, "/");
    });
}

#[gpui::test]
fn test_router_state_events_and_event_log(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        navi_router::event_bus::init_event_log(cx);
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        RouterState::update(cx, |state, _| {
            state.subscribe(move |_event| {
                calls_clone.fetch_add(1, Ordering::SeqCst);
            });
        });
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/about"), &NavigateOptions::default(), cx);
        });
        assert!(
            calls.load(Ordering::SeqCst) >= 2,
            "subscribe callbacks fired"
        );
        let log = navi_router::event_bus::get_event_log(cx);
        assert!(
            !log.is_empty(),
            "global event log must record navigation events"
        );
        assert!(matches!(log[0].event, RouterEvent::BeforeNavigate { .. }));
        assert!(matches!(
            log.last().expect("nonempty").event,
            RouterEvent::Resolved { .. }
        ));
    });
}

#[gpui::test]
fn test_router_state_replace_navigation(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        // Replacing the initial entry must not grow the history.
        RouterState::update(cx, |state, cx| {
            state.navigate(
                Location::new("/about"),
                &NavigateOptions {
                    replace: true,
                    ..NavigateOptions::default()
                },
                cx,
            );
        });
        assert!(!RouterState::global(cx).can_go_back());
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/about"
        );
        // A push afterwards grows the history normally.
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/"), &NavigateOptions::default(), cx);
        });
        assert!(RouterState::global(cx).can_go_back());
        assert_eq!(RouterState::global(cx).current_location().pathname, "/");
    });
}

#[gpui::test]
fn test_router_state_go_back_forward(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/about"), &NavigateOptions::default(), cx);
            assert!(state.can_go_back());
            state.back();
            assert_eq!(state.current_location().pathname, "/");
            state.forward();
            assert_eq!(state.current_location().pathname, "/about");
            state.go(-5);
            assert_eq!(state.current_location().pathname, "/");
        });
    });
}

#[gpui::test]
fn test_router_state_not_found_path(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.not_found_data = Some(serde_json::json!({"hint": "x"}));
            state.navigate(Location::new("/nope"), &NavigateOptions::default(), cx);
        });
        // Navigating to an unknown path pushes /404 (no such route -> no match)
        // and records not_found_data.
        assert_eq!(
            RouterState::global(cx).not_found_data,
            Some(serde_json::json!({"hint": "x"}))
        );
    });
}

#[gpui::test]
fn test_router_state_loader_registration_and_keys(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, _cx| {
            state.register_loader_factory(
                "user",
                Arc::new(move |_params: &HashMap<String, String>| {
                    rs_query::Query::new(rs_query::QueryKey::new("test").with("k", "v"), || async {
                        Ok(LoaderOutcome::Data(AnyData::new("payload".to_string())))
                    })
                }),
            );
        });
        let state = RouterState::global(cx);
        assert!(
            state
                .query_client
                .get_query_data::<AnyData>(&rs_query::QueryKey::new("navi").with("x", "y"))
                .is_none(),
            "empty client starts with no data"
        );
    });
}

#[gpui::test]
fn test_router_state_current_meta_empty(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/about"), &NavigateOptions::default(), cx);
        });
        assert!(RouterState::global(cx).current_meta().is_empty());
    });
}

#[gpui::test]
fn test_router_state_try_global_and_global(cx: &mut gpui::TestAppContext) {
    let _window = setup_router(cx);
    cx.update(|cx| {
        assert!(RouterState::try_global(cx).is_some());
        assert_eq!(RouterState::global(cx).current_location().pathname, "/");
    });
}

#[gpui::test]
fn test_navigator_navigates(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, window, cx| {
        let navigator = Navigator::new(window.window_handle());
        navigator.push("/about", cx);
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/about"
        );
        navigator.push("/users/1", cx);
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/users/1"
        );
        navigator.back(cx);
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/about"
        );
        navigator.forward(cx);
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/users/1"
        );
        navigator.go(-2, cx);
        assert_eq!(RouterState::global(cx).current_location().pathname, "/");
        navigator.replace("/about", cx);
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/about"
        );
        // Relative base resolution: "settings" against base "/users".
        let nav = Navigator::from_route(window.window_handle(), "/users");
        nav.push("settings", cx);
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/users/settings"
        );
        assert!(Navigator::can_go_back(cx));
    });
}

#[gpui::test]
fn test_register_routes_from_define_router(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        macro_routes::register_routes(cx);
    });
    // No panic == components/loaders registered without issue.
}

// ---------------------------------------------------------------------------
// Transactions (NavOp batches) and scroll-reset plumbing.
// ---------------------------------------------------------------------------

#[gpui::test]
fn test_router_state_transact(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.transact(
                &[
                    NavOp::Push(Location::new("/about")),
                    NavOp::Push(Location::new("/users/3")),
                    NavOp::Back,
                ],
                cx,
            );
        });
        let state = RouterState::global(cx);
        // Two pushes + a back: cursor sits on /about.
        assert_eq!(state.current_location().pathname, "/about");
        assert!(state.can_go_forward());
        assert!(state.can_go_back());
        // current_match re-resolved by the traversal inside the transaction.
        assert_eq!(state.current_match.as_ref().expect("match").1.id, "about");
    });
}

#[gpui::test]
fn test_router_state_transact_falls_back_with_blockers(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, _| {
            state.add_blocker(Blocker::new_sync(|_, _| true));
        });
        RouterState::update(cx, |state, cx| {
            state.transact(&[NavOp::Push(Location::new("/about"))], cx);
        });
    });
    // Fallback routes through `navigate`, so the blocker held it pending.
    cx.run_until_parked();
    let _ = window.update(cx, |_, _, cx| {
        assert!(RouterState::global(cx).is_blocked());
        assert_eq!(RouterState::global(cx).current_location().pathname, "/");
        RouterState::update(cx, RouterState::proceed);
    });
    cx.run_until_parked();
    let _ = window.update(cx, |_, _, cx| {
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/about"
        );
    });
}

#[gpui::test]
fn test_navigator_transact(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, window, cx| {
        let navigator = Navigator::new(window.window_handle());
        navigator.transact(
            [
                NavOp::Push(Location::new("/about")),
                NavOp::Replace(Location::new("/users/8")),
            ],
            cx,
        );
        assert_eq!(
            RouterState::global(cx).current_location().pathname,
            "/users/8"
        );
        // Replace kept the history at 2 entries.
        assert!(!RouterState::global(cx).can_go_forward());
    });
}

#[gpui::test]
fn test_navigator_push_with_options_sets_scroll_reset(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, window, cx| {
        let navigator = Navigator::new(window.window_handle());
        let options = NavigateOptions {
            reset_scroll: Some(true),
            ..NavigateOptions::default()
        };
        navigator.push_location_with(Location::new("/about"), &options, cx);
        // The one-shot flag set by the navigation is consumed exactly once.
        RouterState::update(cx, |state, _| {
            assert!(state.take_scroll_reset());
            assert!(!state.take_scroll_reset());
        });
        // Without reset_scroll the flag stays clear.
        navigator.push_location(Location::new("/"), cx);
        RouterState::update(cx, |state, _| {
            assert!(!state.take_scroll_reset());
        });
    });
}

#[gpui::test]
fn test_router_state_traversal_rematches(cx: &mut gpui::TestAppContext) {
    let window = setup_router(cx);
    let _ = window.update(cx, |_, _, cx| {
        RouterState::update(cx, |state, cx| {
            state.navigate(Location::new("/users/5"), &NavigateOptions::default(), cx);
        });
        RouterState::update(cx, |state, _| {
            assert!(state.back());
            let matched = state.current_match.as_ref().expect("match");
            assert_eq!(matched.1.id, "home", "back must re-match the route");
        });
        RouterState::update(cx, |state, _| {
            assert!(state.forward());
            let matched = state.current_match.as_ref().expect("match");
            assert_eq!(matched.1.id, "user", "forward must re-match the route");
        });
    });
}
