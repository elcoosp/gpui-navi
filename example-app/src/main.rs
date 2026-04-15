use gpui::*;
use navi_devtools::DevtoolsState;
use navi_macros::{define_route, use_loader_data};
use navi_router::{
    Location, RouteNode, RoutePattern, RouteTree,
    components::{Link, Outlet, RouterProvider, SuspenseBoundary, register_route_component},
};
use serde::Deserialize;
use std::time::Duration;

// ----------------------------------------------------------------------------
// Root Layout
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct RootLayout;
impl RenderOnce for RootLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_4()
                    .bg(rgb(0x313244))
                    .child(Link::new("/").child("🏠 Home"))
                    .child(Link::new("/about").child("ℹ️ About"))
                    .child(Link::new("/users").child("👥 Users"))
                    .child(Link::new("/users/1").child("User 1"))
                    .child(Link::new("/users/2").child("User 2"))
                    .child(Link::new("/settings").child("⚙️ Settings"))
                    .child(Link::new("/docs/getting-started").child("📄 Docs (splat)")),
            )
            .child(
                SuspenseBoundary::new()
                    .pending_component(div().child("Loading..."))
                    .child(div().flex_1().p_4().child(Outlet::new())),
            )
    }
}

// ----------------------------------------------------------------------------
// Home Page
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct HomePage;
impl RenderOnce for HomePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child("🏠 Welcome to Navi Router!")
            .child("This demo showcases nested layouts, dynamic routes, loaders, and more.")
    }
}

// ----------------------------------------------------------------------------
// About Page
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct AboutPage;
impl RenderOnce for AboutPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .child("ℹ️ About Navi Router")
            .child("A powerful file‑based router for GPUI with loaders, suspense, and devtools.")
    }
}

// ----------------------------------------------------------------------------
// Users Layout
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct UsersLayout;
impl RenderOnce for UsersLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child("👥 Users Section")
            .child(div().flex_1().child(Outlet::new()))
    }
}

// ----------------------------------------------------------------------------
// Users Index
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct UsersIndexPage;
impl RenderOnce for UsersIndexPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child("Select a user:")
            .child(Link::new("/users/1").child("User 1"))
            .child(Link::new("/users/2").child("User 2"))
            .child(Link::new("/users/42").child("User 42"))
    }
}

// ----------------------------------------------------------------------------
// User Detail Route – Declarative Loader
// ----------------------------------------------------------------------------
#[derive(Clone, Debug, Deserialize)]
pub struct UserParams {
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct UserData {
    pub id: String,
    pub name: String,
    pub email: String,
}

define_route!(
    UserDetailRoute,
    path: "/users/$id",
    params: UserParams,
    data: UserData,
    loader: |params: UserParams, executor: gpui::BackgroundExecutor| async move {
        let id = params.id;
        executor.timer(Duration::from_millis(800)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(UserData {
            id: id.clone(),
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        }))
    },
    component: UserDetailPage,
);

#[derive(Clone, IntoElement)]
struct UserDetailPage;
impl RenderOnce for UserDetailPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = use_loader_data!(UserDetailRoute);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!("User ID: {}", data.id))
            .child(format!("Name: {}", data.name))
            .child(format!("Email: {}", data.email))
            .into_any_element()
    }
}

// ----------------------------------------------------------------------------
// Settings Page
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct SettingsPage;
impl RenderOnce for SettingsPage {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .child("⚙️ Settings Page")
            .child("Configure your application here.")
    }
}

// ----------------------------------------------------------------------------
// Docs Splat Route
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct DocsSplatPage;
impl RenderOnce for DocsSplatPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = navi_router::RouterState::global(cx);
        let path = state.current_location().pathname;
        let slug = path.strip_prefix("/docs/").unwrap_or("unknown");
        div()
            .child(format!("📄 Documentation: {}", slug))
            .child("This is a catch‑all splat route.")
    }
}

// ----------------------------------------------------------------------------
// Not Found Page
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct NotFoundPage;
impl RenderOnce for NotFoundPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child("404 – Page Not Found")
    }
}

// ----------------------------------------------------------------------------
// Root View
// ----------------------------------------------------------------------------
struct AppView {
    router_provider: RouterProvider,
    devtools: Entity<DevtoolsState>,
}
impl Render for AppView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .child(self.router_provider.clone().child(RootLayout))
            .child(self.devtools.clone())
    }
}

// ----------------------------------------------------------------------------
// Main
// ----------------------------------------------------------------------------
fn main() {
    register_route_component("__root__", |_| {
        Component::new(RootLayout).into_any_element()
    });
    register_route_component("index", |_| Component::new(HomePage).into_any_element());
    register_route_component("about", |_| Component::new(AboutPage).into_any_element());
    register_route_component("users", |_| Component::new(UsersLayout).into_any_element());
    register_route_component("users_index", |_| {
        Component::new(UsersIndexPage).into_any_element()
    });
    register_route_component("UserDetailRoute", |_| {
        Component::new(UserDetailPage).into_any_element()
    });
    register_route_component("settings", |_| {
        Component::new(SettingsPage).into_any_element()
    });
    register_route_component("docs_splat", |_| {
        Component::new(DocsSplatPage).into_any_element()
    });
    register_route_component("not_found", |_| {
        Component::new(NotFoundPage).into_any_element()
    });

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
            parent: Some("__root__".into()),
            is_layout: false,
            is_index: true,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
        });

        tree.add_route(RouteNode {
            id: "about".to_string(),
            pattern: RoutePattern::parse("/about"),
            parent: Some("__root__".into()),
            is_layout: false,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
        });

        tree.add_route(RouteNode {
            id: "users".to_string(),
            pattern: RoutePattern::parse("/users"),
            parent: Some("__root__".into()),
            is_layout: true,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
        });

        tree.add_route(RouteNode {
            id: "users_index".to_string(),
            pattern: RoutePattern::parse("/users"),
            parent: Some("users".into()),
            is_layout: false,
            is_index: true,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
        });

        tree.add_route(UserDetailRoute::build_node());

        tree.add_route(RouteNode {
            id: "settings".to_string(),
            pattern: RoutePattern::parse("/settings"),
            parent: Some("__root__".into()),
            is_layout: false,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
        });

        tree.add_route(RouteNode {
            id: "docs_splat".to_string(),
            pattern: RoutePattern::parse("/docs/$"),
            parent: Some("__root__".into()),
            is_layout: false,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
        });

        tree.add_route(RouteNode {
            id: "not_found".to_string(),
            pattern: RoutePattern::parse("/*"),
            parent: Some("__root__".into()),
            is_layout: false,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
        });

        let devtools = cx.new(|_cx| DevtoolsState::new());

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(900.0), px(700.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                let window_id = window.window_handle().window_id();
                let initial = Location::new("/");
                let router_provider = RouterProvider::new(window_id, initial, tree, cx);

                // Register loader AFTER the global RouterState has been initialized by RouterProvider
                UserDetailRoute::register_loader(cx);

                cx.new(|_cx| AppView {
                    router_provider,
                    devtools,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
