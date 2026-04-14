use gpui::WindowId;
use navi_router::{
    Location, RouterState,
    RouteNode, RoutePattern,
};
use navi_router::components::{RouterProvider, Outlet, Link};

mod routes;

fn main() {
    // Build the router with initial location
    let window_id = WindowId(0);
    let initial = Location::new("/");
    let mut tree = navi_router::RouteTree::new();

    // Register routes
    tree.add_route(RouteNode {
        id: "__root__".to_string(),
        pattern: RoutePattern::parse("/"),
        parent: None,
        is_layout: true,
        is_index: false,
        has_loader: false,
        loader_stale_time: None,
        loader_gc_time: None,
        preload_stale_time: None,
    });

    tree.add_route(RouteNode {
        id: "index".to_string(),
        pattern: RoutePattern::parse("/"),
        parent: Some("__root__".to_string()),
        is_layout: false,
        is_index: true,
        has_loader: false,
        loader_stale_time: None,
        loader_gc_time: None,
        preload_stale_time: None,
    });

    tree.add_route(RouteNode {
        id: "users_index".to_string(),
        pattern: RoutePattern::parse("/users"),
        parent: Some("__root__".to_string()),
        is_layout: false,
        is_index: false,
        has_loader: false,
        loader_stale_time: None,
        loader_gc_time: None,
        preload_stale_time: None,
    });

    tree.add_route(RouteNode {
        id: "user_detail".to_string(),
        pattern: RoutePattern::parse("/users/$id"),
        parent: Some("__root__".to_string()),
        is_layout: false,
        is_index: false,
        has_loader: true,
        loader_stale_time: Some(std::time::Duration::from_secs(30)),
        loader_gc_time: Some(std::time::Duration::from_secs(300)),
        preload_stale_time: Some(std::time::Duration::from_secs(30)),
    });

    tree.add_route(RouteNode {
        id: "settings".to_string(),
        pattern: RoutePattern::parse("/settings"),
        parent: Some("__root__".to_string()),
        is_layout: false,
        is_index: false,
        has_loader: false,
        loader_stale_time: None,
        loader_gc_time: None,
        preload_stale_time: None,
    });

    // Display registered routes
    println!("Navi Example App - Router initialized successfully!");
    println!("Registered routes:");
    for node in tree.all_nodes() {
        println!("  {} -> {}", node.id, node.pattern.raw);
    }

    // Test route matching before moving tree
    let test_paths = ["/", "/users", "/users/42", "/settings", "/unknown"];
    for path in test_paths {
        match tree.match_path(path) {
            Some((params, node)) => {
                println!("Matched {} -> {} ({:?})", path, node.id, params);
            }
            None => {
                println!("No match for {}", path);
            }
        }
    }

    let _router_state = RouterState::new(initial, window_id, tree);

    // Create the app with devtools
    let _devtools = navi_devtools::NaviDevtools::new()
        .selected_tab(navi_devtools::devtools::DevtoolsTab::Routes);

    println!("\nRouter state created successfully. Navigation system ready.");
}
