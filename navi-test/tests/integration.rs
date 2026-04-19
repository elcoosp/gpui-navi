use gpui::*;
use navi_router::*;
use std::collections::HashMap;

#[gpui::test]
async fn test_route_matching() {
    let mut tree = RouteTree::new();
    let node = RouteNode {
        id: "test".to_string(),
        pattern: RoutePattern::parse("/test"),
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
    };
    tree.add_route(node);
    let (params, matched) = tree.match_path("/test").unwrap();
    assert_eq!(matched.id, "test");
    assert!(params.is_empty());
}

#[test]
fn test_navigation_blocker_sync() {
    let blocker = Blocker::new_sync(|_, _| true);
    let from = Location::new("/from");
    let to = Location::new("/to");
    let allow = futures::executor::block_on(blocker.should_allow(&from, &to));
    assert!(!allow);
}
