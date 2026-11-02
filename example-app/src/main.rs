use gpui::{
    App, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use gpui_kit::assets::Assets;
use navi_devtools::DevtoolsState;
use navi_router::{
    Location, NotFoundMode, RouterOptions, RouterState,
    components::{Outlet, RouterProvider},
};

mod route_tree {
    include!("route_tree.gen.rs");
}
use route_tree::build_route_tree;

struct AppView {
    router_provider: RouterProvider,
    devtools: Entity<DevtoolsState>,
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Dialog, sheet and notification layers are mounted by the kit's
        // Root itself in the post-restructure stack; the app view renders
        // only its own content.
        div()
            .size_full()
            .relative()
            .child(self.router_provider.clone().child(Outlet::new()))
            .child(self.devtools.clone())
    }
}

fn main() {
    env_logger::init();

    gpui_kit::application()
        .with_assets(Assets)
        .run(|cx: &mut App| {
            cx.init_colors();
            gpui_kit::init(cx);

            // Validate route tree for cycles
            let tree = build_route_tree();
            for node in tree.all_nodes() {
                let ancestors = tree.ancestors(&node.id);
                if ancestors.len() > 50 {
                    eprintln!(
                        "Warning: Deep ancestor chain for {} (len {}): {:?}",
                        node.id,
                        ancestors.len(),
                        ancestors.iter().map(|n| n.id.clone()).collect::<Vec<_>>()
                    );
                }
            }
            eprintln!("Route tree validation complete.");

            let tree = build_route_tree();

            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(900.0), px(700.0)),
                        cx,
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    let window_id = window.window_handle().window_id();
                    let window_handle = window.window_handle();
                    let initial = Location::new("/");

                    let router_provider = RouterProvider::new_with_options(
                        window_id,
                        window_handle,
                        initial,
                        tree,
                        RouterOptions {
                            default_pending_ms: 500,
                            default_pending_min_ms: 200,
                            not_found_mode: NotFoundMode::Fuzzy,
                        },
                        cx,
                    );

                    route_tree::register_routes(cx);

                    let query_client = RouterState::global(cx).query_client.clone();
                    let devtools = cx.new(|cx| DevtoolsState::new(query_client, cx));

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
        });
}
