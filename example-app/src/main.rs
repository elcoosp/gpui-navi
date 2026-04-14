use gpui::*;
use navi_core::suspense::SuspenseState;
use navi_devtools::devtools::DevtoolsState;
use navi_router::{
    Location, RouteNode, RoutePattern, RouteTree, RouterState,
    components::{Link, Outlet, RouterProvider, register_route_component},
};
use std::sync::Arc;
use std::time::Duration;

// ----------------------------------------------------------------------------
// Route Components
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
                    .child(Link::new("/").child("Home"))
                    .child(Link::new("/about").child("About"))
                    .child(Link::new("/users/1").child("User 1"))
                    .child(Link::new("/users/2").child("User 2")),
            )
            .child(div().flex_1().p_4().child(Outlet::new()))
    }
}

#[derive(Clone, IntoElement)]
struct HomePage;
impl RenderOnce for HomePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Welcome Home!")
    }
}

#[derive(Clone, IntoElement)]
struct AboutPage;
impl RenderOnce for AboutPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("About this app")
    }
}

// ----------------------------------------------------------------------------
// User Detail with Async Loader
// ----------------------------------------------------------------------------

#[derive(Clone)]
struct UserData {
    id: String,
    name: String,
    email: String,
}

struct UserLoader {
    state: SuspenseState<Arc<UserData>>,
}

impl UserLoader {
    fn new(_user_id: String, cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| Self {
            state: SuspenseState::Idle,
        })
    }

    fn load(&mut self, user_id: String, cx: &mut Context<Self>) {
        if !matches!(self.state, SuspenseState::Idle) {
            return;
        }

        self.state = SuspenseState::Pending;
        cx.notify();

        cx.spawn(async move |this, async_app| {
            async_app
                .background_executor()
                .timer(Duration::from_millis(1000))
                .await;

            let data = Arc::new(UserData {
                id: user_id.clone(),
                name: format!("User {}", user_id),
                email: format!("user{}@example.com", user_id),
            });

            if let Err(e) = this.update(async_app, |this, cx| {
                this.state = SuspenseState::Ready(data);
                cx.notify();
            }) {
                eprintln!("ERROR updating UserLoader: {:?}", e);
            }
        })
        .detach();
    }
}

impl Render for UserLoader {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        match &self.state {
            SuspenseState::Idle => div().child("IDLE").into_any_element(),
            SuspenseState::Pending => div().child("LOADING...").into_any_element(),
            SuspenseState::Ready(data) => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(format!("User ID: {}", data.id))
                .child(format!("Name: {}", data.name))
                .child(format!("Email: {}", data.email))
                .into_any_element(),
            SuspenseState::Error(e) => div().child(format!("Error: {}", e)).into_any_element(),
        }
    }
}

// ----------------------------------------------------------------------------
// Cached Loader State
// ----------------------------------------------------------------------------

#[derive(Clone)]
struct LoaderState {
    user_id: String,
    loader: Entity<UserLoader>,
}

#[derive(Clone, IntoElement)]
struct UserDetailPage;

impl RenderOnce for UserDetailPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let user_id = RouterState::try_global(cx)
            .and_then(|state| state.current_match.as_ref())
            .and_then(|(params, _)| params.get("id").cloned())
            .unwrap_or_else(|| "unknown".to_string());

        let element_id = ElementId::Name("user-detail-loader".into());

        window.with_global_id(element_id, |global_id, window| {
            window.with_element_state::<LoaderState, _>(global_id, |mut existing_state, _window| {
                let needs_new = match existing_state {
                    None => true,
                    Some(ref s) => s.user_id != user_id,
                };

                let state = if needs_new {
                    let loader = UserLoader::new(user_id.clone(), cx);
                    loader.update(cx, |loader, cx| loader.load(user_id.clone(), cx));
                    LoaderState { user_id, loader }
                } else {
                    existing_state.take().unwrap()
                };

                let loader = state.loader.clone();

                (div().child(loader), state)
            })
        })
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
            .relative() // Ensure absolute positioning context for devtools
            .child(self.router_provider.clone().child(RootLayout))
            .child(self.devtools.clone())
    }
}

// ----------------------------------------------------------------------------
// Main
// ----------------------------------------------------------------------------

fn main() {
    register_route_component("index", |_| Component::new(HomePage).into_any_element());
    register_route_component("about", |_| Component::new(AboutPage).into_any_element());
    register_route_component("user_detail", |_| {
        Component::new(UserDetailPage).into_any_element()
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
            id: "user_detail".to_string(),
            pattern: RoutePattern::parse("/users/$id"),
            parent: Some("__root__".into()),
            is_layout: false,
            is_index: false,
            has_loader: true,
            loader_stale_time: Some(Duration::from_secs(30)),
            loader_gc_time: Some(Duration::from_secs(300)),
            preload_stale_time: Some(Duration::from_secs(30)),
        });

        let devtools = cx.new(|_cx| DevtoolsState::new());

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
