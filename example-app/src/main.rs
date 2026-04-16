use gpui::prelude::*;
use gpui::*;
use gpui_component::Root;
use gpui_component::scroll::ScrollableElement;
use gpui_component_assets::Assets;
use navi_devtools::DevtoolsState;
use navi_macros::{define_route, use_loader_data, use_search};
use navi_router::{
    Blocker, Location, Navigator, RouteNode, RoutePattern, RouteTree, RouterState, ValidateSearch,
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
                    .child(Link::new("/docs/getting-started").child("📄 Docs (splat)"))
                    .child(Link::new("/validation-test").child("🧪 Validation Tests")),
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
            .overflow_y_scrollbar()
            .gap_6()
            .p_8()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child("🏠 Welcome to Navi Router!"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xa6adc8))
                    .child("This demo showcases nested layouts, dynamic routes, loaders, and more."),
            )
            .child(
                div()
                    .max_w(px(600.0))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .text_color(rgb(0xbac2de))
                    .child(
                        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat."
                    )
                    .child(
                        "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
                    )
                    .child(
                        "Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam varius, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in mauris eu nibh euismod gravida."
                    )
                    .child(
                        "Duis ac tellus et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor. Ut ullamcorper, ligula ut dictum pharetra, nisi nunc fringilla magna, in commodo elit erat nec turpis. Ut pharetra augue nec augue."
                    )
                    .child(
                        "Nam dui ligula, fringilla a, euismod sodales, sollicitudin vel, wisi. Morbi auctor lorem non justo. Nam lacus libero, pretium at, lobortis vitae, ultricies et, tellus. Donec aliquet, tortor sed accumsan bibendum, erat ligula aliquet magna, vitae ornare odio metus a mi."
                    )
                    .child(
                        "Morbi ac orci et nisl hendrerit mollis. Suspendisse ut massa. Cras nec ante. Pellentesque a nulla. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Aliquam tincidunt urna."
                    )
                    .child(
                        "Nulla ullamcorper vestibulum turpis. Pellentesque cursus luctus mauris. Nulla malesuada porttitor diam. Donec felis erat, congue non, volutpat at, tincidunt tristique, libero. Vivamus viverra fermentum felis."
                    )
            )
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
// Users Index (with search params)
// ----------------------------------------------------------------------------
define_route!(
    UsersIndexRoute,
    path: "/users",
    search: UsersSearch,
    is_index: true,
    component: UsersIndexPage,
);

#[derive(Clone, IntoElement)]
struct UsersIndexPage;
impl RenderOnce for UsersIndexPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let search = use_search!(UsersIndexRoute);
        let navigator = Navigator::new(window.window_handle());

        let current_sort = search.sort.unwrap_or_default();
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
// Settings Page (with Navigation Blocker Demo)
// ----------------------------------------------------------------------------
#[derive(Clone, IntoElement)]
struct SettingsPage;
impl RenderOnce for SettingsPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let block_navigation = cx.new(|cx| {
            let focus_handle = cx.focus_handle();
            focus_handle.focus(window, cx);
            BlockerState {
                block: false,
                focus_handle,
            }
        });

        div()
            .child("⚙️ Settings Page")
            .child("Configure your application here.")
            .child(
                div().flex().gap_2().child("Block navigation:").child(
                    div()
                        .cursor_pointer()
                        .child(if block_navigation.read(cx).block {
                            "✅"
                        } else {
                            "⬜"
                        })
                        .on_mouse_up(MouseButton::Left, {
                            let block_navigation = block_navigation.clone();
                            move |_event, _window, cx| {
                                block_navigation.update(cx, |state, cx| {
                                    state.block = !state.block;
                                    cx.notify();
                                });
                            }
                        }),
                ),
            )
            .child("When checked, navigating away will be blocked.")
    }
}

struct BlockerState {
    block: bool,
    #[allow(dead_code)]
    focus_handle: FocusHandle,
}

impl Render for BlockerState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.block {
            RouterState::update(cx, |state, _| {
                state.add_blocker(Blocker::new(move |_from, _to| true));
            });
        }
        div()
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
// Validation Test Pages (conditionally compiled)
// ----------------------------------------------------------------------------

#[cfg(feature = "validator")]
mod validator_test {
    use super::*;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate, Clone, Default)]
    pub struct ValidatorSearch {
        #[validate(range(min = 1, max = 100))]
        pub page: Option<u32>,
        #[validate(length(min = 1, max = 10))]
        pub sort: Option<String>,
    }

    define_route!(
        ValidatorTestRoute,
        path: "/validation-test/validator",
        search: ValidatorSearch,
        component: ValidatorTestPage,
    );

    #[derive(Clone, IntoElement)]
    pub struct ValidatorTestPage;

    impl RenderOnce for ValidatorTestPage {
        fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
            let raw = use_search!(ValidatorTestRoute);
            let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
            div()
                .p_4()
                .child("Validator Test Page")
                .child(format!(
                    "Page: {:?}, Sort: {:?}",
                    validated.page, validated.sort
                ))
                .child(
                    Link::new("/validation-test/validator?page=50&sort=desc")
                        .child("Valid params (page=50, sort=desc)"),
                )
                .child(
                    Link::new("/validation-test/validator?page=999&sort=invalid")
                        .child("Invalid params (should fallback to default)"),
                )
                .child(Link::new("/validation-test").child("Back to validation index"))
        }
    }
}

#[cfg(feature = "garde")]
mod garde_test {
    use super::*;
    use garde::Validate as GardeValidate;

    #[derive(Debug, Deserialize, Clone, GardeValidate, Default)]
    #[garde(allow_unvalidated)]
    pub struct GardeSearch {
        #[garde(range(min = 1, max = 100))]
        pub page: Option<u32>,
        #[garde(length(min = 1, max = 10))]
        pub sort: Option<String>,
    }

    define_route!(
        GardeTestRoute,
        path: "/validation-test/garde",
        search: GardeSearch,
        component: GardeTestPage,
    );

    #[derive(Clone, IntoElement)]
    pub struct GardeTestPage;

    impl RenderOnce for GardeTestPage {
        fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
            let raw = use_search!(GardeTestRoute);
            let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
            div()
                .p_4()
                .child("Garde Test Page")
                .child(format!(
                    "Page: {:?}, Sort: {:?}",
                    validated.page, validated.sort
                ))
                .child(
                    Link::new("/validation-test/garde?page=50&sort=desc")
                        .child("Valid params (page=50, sort=desc)"),
                )
                .child(
                    Link::new("/validation-test/garde?page=999&sort=invalid")
                        .child("Invalid params (should fallback to default)"),
                )
                .child(Link::new("/validation-test").child("Back to validation index"))
        }
    }
}

#[cfg(feature = "validify")]
mod validify_test {
    use super::*;
    use validify::Validate;

    #[derive(Debug, Deserialize, Validate, Clone, Default)]
    pub struct ValidifySearch {
        #[validate(range(min = 1, max = 100))]
        pub page: Option<u32>,
        #[validate(length(min = 1, max = 10))]
        pub sort: Option<String>,
    }

    define_route!(
        ValidifyTestRoute,
        path: "/validation-test/validify",
        search: ValidifySearch,
        component: ValidifyTestPage,
    );

    #[derive(Clone, IntoElement)]
    pub struct ValidifyTestPage;

    impl RenderOnce for ValidifyTestPage {
        fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
            let raw = use_search!(ValidifyTestRoute);
            let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
            div()
                .p_4()
                .child("Validify Test Page")
                .child(format!(
                    "Page: {:?}, Sort: {:?}",
                    validated.page, validated.sort
                ))
                .child(
                    Link::new("/validation-test/validify?page=50&sort=desc")
                        .child("Valid params (page=50, sort=desc)"),
                )
                .child(
                    Link::new("/validation-test/validify?page=999&sort=invalid")
                        .child("Invalid params (should fallback to default)"),
                )
                .child(Link::new("/validation-test").child("Back to validation index"))
        }
    }
}

#[cfg(feature = "valico")]
mod valico_test {
    use super::*;
    use schemars::JsonSchema;

    #[derive(Debug, Deserialize, Default, Clone, JsonSchema)]
    pub struct ValicoSearch {
        pub page: Option<u32>,
        pub sort: Option<String>,
    }

    impl ValicoSearch {
        fn validate(&self) -> Result<(), String> {
            if let Some(p) = self.page {
                if !(1..=100).contains(&p) {
                    return Err("page out of range".into());
                }
            }
            if let Some(s) = &self.sort {
                if s.len() > 10 {
                    return Err("sort too long".into());
                }
            }
            Ok(())
        }
    }

    define_route!(
        ValicoTestRoute,
        path: "/validation-test/valico",
        search: ValicoSearch,
        component: ValicoTestPage,
    );

    #[derive(Clone, IntoElement)]
    pub struct ValicoTestPage;

    impl RenderOnce for ValicoTestPage {
        fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
            let raw = use_search!(ValicoTestRoute);
            let validated = raw.validate().ok().map(|_| raw).unwrap_or_default();
            div()
                .p_4()
                .child("Valico Test Page")
                .child(format!(
                    "Page: {:?}, Sort: {:?}",
                    validated.page, validated.sort
                ))
                .child(
                    Link::new("/validation-test/valico?page=50&sort=desc")
                        .child("Valid params (page=50, sort=desc)"),
                )
                .child(
                    Link::new("/validation-test/valico?page=999&sort=invalid")
                        .child("Invalid params (should fallback to default)"),
                )
                .child(Link::new("/validation-test").child("Back to validation index"))
        }
    }
}

// Validation Test Index Page
#[derive(Clone, IntoElement)]
struct ValidationTestIndex;

impl RenderOnce for ValidationTestIndex {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .p_4()
            .child("Validation Test Index")
            .child("Select a validation crate to test:")
            .child(Link::new("/validation-test/validator").child("validator"))
            .child(Link::new("/validation-test/garde").child("garde"))
            .child(Link::new("/validation-test/validify").child("validify"))
            .child(Link::new("/validation-test/valico").child("valico"))
            .child(Link::new("/").child("Home"))
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        log::debug!("AppView rendered");
        div()
            .size_full()
            .relative()
            .child(self.router_provider.clone().child(RootLayout))
            .child(self.devtools.clone())
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

// ----------------------------------------------------------------------------
// Main
// ----------------------------------------------------------------------------
fn main() {
    env_logger::init();
    log::info!("Starting Navi example app");

    register_route_component("__root__", |_| {
        Component::new(RootLayout).into_any_element()
    });
    register_route_component("index", |_| Component::new(HomePage).into_any_element());
    register_route_component("about", |_| Component::new(AboutPage).into_any_element());
    register_route_component("users", |_| Component::new(UsersLayout).into_any_element());
    register_route_component("UsersIndexRoute", |_| {
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
    register_route_component("validation_index", |_| {
        Component::new(ValidationTestIndex).into_any_element()
    });

    #[cfg(feature = "validator")]
    register_route_component("ValidatorTestRoute", |_| {
        Component::new(validator_test::ValidatorTestPage).into_any_element()
    });
    #[cfg(feature = "garde")]
    register_route_component("GardeTestRoute", |_| {
        Component::new(garde_test::GardeTestPage).into_any_element()
    });
    #[cfg(feature = "validify")]
    register_route_component("ValidifyTestRoute", |_| {
        Component::new(validify_test::ValidifyTestPage).into_any_element()
    });
    #[cfg(feature = "valico")]
    register_route_component("ValicoTestRoute", |_| {
        Component::new(valico_test::ValicoTestPage).into_any_element()
    });

    gpui_platform::application()
        .with_assets(Assets)
        .run(|cx: &mut App| {
            cx.init_colors();
            gpui_component::init(cx);
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

            tree.add_route(RouteNode {
                id: "validation_index".to_string(),
                pattern: RoutePattern::parse("/validation-test"),
                parent: Some("__root__".into()),
                is_layout: false,
                is_index: true,
                has_loader: false,
                loader_stale_time: None,
                loader_gc_time: None,
                preload_stale_time: None,
            });

            #[cfg(feature = "validator")]
            {
                let mut node = validator_test::ValidatorTestRoute::build_node();
                node.parent = Some("__root__".to_string());
                tree.add_route(node);
            }
            #[cfg(feature = "garde")]
            {
                let mut node = garde_test::GardeTestRoute::build_node();
                node.parent = Some("__root__".to_string());
                tree.add_route(node);
            }
            #[cfg(feature = "validify")]
            {
                let mut node = validify_test::ValidifyTestRoute::build_node();
                node.parent = Some("__root__".to_string());
                tree.add_route(node);
            }
            #[cfg(feature = "valico")]
            {
                let mut node = valico_test::ValicoTestRoute::build_node();
                node.parent = Some("__root__".to_string());
                tree.add_route(node);
            }

            let devtools = cx.new(|cx| DevtoolsState::new(cx));

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

                    UserDetailRoute::register_loader(cx);

                    let root_view = cx.new(|_cx| AppView {
                        router_provider,
                        devtools,
                    });

                    RouterState::update(cx, |state, _| state.set_root_view(root_view.entity_id()));

                    cx.new(|cx| Root::new(root_view, window, cx))
                },
            )
            .unwrap();

            cx.activate(true);
            log::info!("Application running");
        });
}
