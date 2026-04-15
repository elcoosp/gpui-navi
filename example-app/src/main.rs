use gpui::prelude::*;
use gpui::*;
use navi_devtools::DevtoolsState;
use navi_macros::{define_route, use_loader_data, use_search};
use navi_router::RouteDef;
use navi_router::{
    Location, Navigator, RouteNode, RoutePattern, RouteTree, RouterState, ValidateSearch,
    ValidationError, ValidationResult,
    components::{Link, Outlet, RouterProvider, register_route_component},
};
use serde::Deserialize;
use std::collections::HashMap;
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
            .child(div().flex_1().p_4().child(Outlet::new()))
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
        log::debug!("UserDetailRoute loader started for id: {}", params.id);
        let id = params.id;
        executor.timer(Duration::from_millis(800)).await;
        log::debug!("UserDetailRoute loader completed for id: {}", id);
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
        match data {
            Some(data) => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(format!("User ID: {}", data.id))
                .child(format!("Name: {}", data.name))
                .child(format!("Email: {}", data.email))
                .into_any_element(),
            None => div().child("Loading user data...").into_any_element(),
        }
    }
}

// ----------------------------------------------------------------------------
// Search Params for Users Index
// ----------------------------------------------------------------------------
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

impl std::fmt::Display for SortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortDirection::Asc => write!(f, "asc"),
            SortDirection::Desc => write!(f, "desc"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct UsersSearch {
    pub sort: Option<SortDirection>,
}

impl ValidateSearch for UsersSearch {
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self> {
        let sort = if let Some(s) = raw.get("sort") {
            match s.as_str() {
                "asc" => Some(SortDirection::Asc),
                "desc" => Some(SortDirection::Desc),
                _ => {
                    return Err(vec![ValidationError {
                        field: Some("sort".to_string()),
                        message: "Invalid sort direction, must be 'asc' or 'desc'".to_string(),
                    }]);
                }
            }
        } else {
            None
        };
        Ok(UsersSearch { sort })
    }

    fn to_query(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(sort) = &self.sort {
            map.insert("sort".to_string(), sort.to_string());
        }
        map
    }
}

// ----------------------------------------------------------------------------
// Users Index Route (manually defined as index route)
// ----------------------------------------------------------------------------
define_route!(
    UsersIndexRoute,
    path: "/users",
    search: UsersSearch,
    is_index: true,          // Mark as index route
    component: UsersIndexPage,
);
// ----------------------------------------------------------------------------
// Users Index Page Component
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct UsersIndexPage;
impl RenderOnce for UsersIndexPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let search = use_search!(UsersIndexRoute);
        let navigator = Navigator::new(window.window_handle());

        let current_sort = search.sort.unwrap_or_default();

        // Generate and sort user IDs
        let mut user_ids = vec![1, 2, 42];
        match current_sort {
            SortDirection::Asc => user_ids.sort(),
            SortDirection::Desc => user_ids.sort_by(|a, b| b.cmp(a)),
        }

        let toggle_sort_asc = {
            let navigator = navigator.clone();
            move |_event: &MouseUpEvent, _window: &mut Window, cx: &mut App| {
                let params = UsersSearch {
                    sort: Some(SortDirection::Asc),
                }
                .to_query();
                let query_string = params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("&");
                let new_path = if query_string.is_empty() {
                    "/users".to_string()
                } else {
                    format!("/users?{}", query_string)
                };
                navigator.push(new_path, cx);
            }
        };
        let toggle_sort_desc = {
            let navigator = navigator.clone();
            move |_event: &MouseUpEvent, _window: &mut Window, cx: &mut App| {
                let params = UsersSearch {
                    sort: Some(SortDirection::Desc),
                }
                .to_query();
                let query_string = params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("&");
                let new_path = if query_string.is_empty() {
                    "/users".to_string()
                } else {
                    format!("/users?{}", query_string)
                };
                navigator.push(new_path, cx);
            }
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child("Select a user (sorting by ID):")
            .child(format!("Current sort: {:?}", current_sort))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(if current_sort == SortDirection::Asc {
                                rgb(0x2563eb)
                            } else {
                                rgb(0x6b7280)
                            })
                            .text_color(white())
                            .rounded_md()
                            .cursor_pointer()
                            .child("↑ Ascending")
                            .on_mouse_up(MouseButton::Left, toggle_sort_asc),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(if current_sort == SortDirection::Desc {
                                rgb(0x2563eb)
                            } else {
                                rgb(0x6b7280)
                            })
                            .text_color(white())
                            .rounded_md()
                            .cursor_pointer()
                            .child("↓ Descending")
                            .on_mouse_up(MouseButton::Left, toggle_sort_desc),
                    ),
            )
            .children(
                user_ids
                    .into_iter()
                    .map(|id| Link::new(format!("/users/{}", id)).child(format!("User {}", id))),
            )
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
        let path = RouterState::try_global(cx)
            .map(|s| s.current_location().pathname)
            .unwrap_or_default();
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
        log::debug!("AppView rendered");
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
    env_logger::init();
    log::info!("Starting Navi example app");

    register_route_component("__root__", |_| {
        log::debug!("Creating RootLayout component");
        Component::new(RootLayout).into_any_element()
    });
    register_route_component("index", |_| {
        log::debug!("Creating HomePage component");
        Component::new(HomePage).into_any_element()
    });
    register_route_component("about", |_| {
        log::debug!("Creating AboutPage component");
        Component::new(AboutPage).into_any_element()
    });
    register_route_component("users", |_| {
        log::debug!("Creating UsersLayout component");
        Component::new(UsersLayout).into_any_element()
    });
    register_route_component("UsersIndexRoute", |_| {
        log::debug!("Creating UsersIndexPage component");
        Component::new(UsersIndexPage).into_any_element()
    });
    register_route_component("UserDetailRoute", |_| {
        log::debug!("Creating UserDetailPage component");
        Component::new(UserDetailPage).into_any_element()
    });
    register_route_component("settings", |_| {
        log::debug!("Creating SettingsPage component");
        Component::new(SettingsPage).into_any_element()
    });
    register_route_component("docs_splat", |_| {
        log::debug!("Creating DocsSplatPage component");
        Component::new(DocsSplatPage).into_any_element()
    });
    register_route_component("not_found", |_| {
        log::debug!("Creating NotFoundPage component");
        Component::new(NotFoundPage).into_any_element()
    });

    Application::new().run(|cx: &mut App| {
        log::info!("Building route tree");
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

        // Add the index route with is_index: true
        tree.add_route(UsersIndexRoute::build_node());

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

        log::info!("Opening window");
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
                let window_handle = window.window_handle();
                let initial = Location::new("/");
                log::info!("Creating RouterProvider with initial location: /");
                let router_provider =
                    RouterProvider::new(window_id, window_handle, initial, tree, cx);

                // Register loader AFTER the global RouterState has been initialized by RouterProvider
                UserDetailRoute::register_loader(cx);

                let root_view = cx.new(|_cx| AppView {
                    router_provider,
                    devtools,
                });

                RouterState::update(cx, |state, _| state.set_root_view(root_view.entity_id()));

                root_view
            },
        )
        .unwrap();

        cx.activate(true);
        log::info!("Application running");
    });
}
