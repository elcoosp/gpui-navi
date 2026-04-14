use gpui::*;
use navi_router::{
    Location, RouteNode, RoutePattern, RouteTree,
    components::{Link, Outlet, RouterProvider, register_route_component},
};
use std::time::Duration;

struct AppView {
    router_provider: RouterProvider,
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_4()
                    .bg(rgb(0xf0f0f0))
                    .child(Link::new("/").child("Home"))
                    .child(Link::new("/users").child("Users"))
                    .child(Link::new("/settings").child("Settings")),
            )
            .child(div().flex_1().p_4().child(Outlet::new()))
    }
}

struct HomePage;
impl Render for HomePage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child("Welcome Home!")
    }
}

struct UsersPage;
impl Render for UsersPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child("Users list")
    }
}

struct SettingsPage;
impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child("Settings")
    }
}

fn main() {
    register_route_component("index", |cx: &mut App| cx.new(|_| HomePage));
    register_route_component("users_index", |cx: &mut App| cx.new(|_| UsersPage));
    register_route_component("settings", |cx: &mut App| cx.new(|_| SettingsPage));

    Application::new().run(|cx: &mut App| {
        let mut tree = RouteTree::new();

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
            loader_stale_time: Some(Duration::from_secs(30)),
            loader_gc_time: Some(Duration::from_secs(300)),
            preload_stale_time: Some(Duration::from_secs(30)),
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

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(800.0), px(600.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                let window_id = window.window_handle().window_id();
                let initial = Location::new("/");

                let router_provider = RouterProvider::new(window_id, initial, tree, cx);

                cx.new(|_cx| AppView { router_provider })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
