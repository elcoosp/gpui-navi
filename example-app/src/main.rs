use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .child(self.router_provider.clone().child(Outlet::new()))
            .child(self.devtools.clone())
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

fn main() {
    env_logger::init();

    gpui_platform::application()
        .with_assets(Assets)
        .run(|cx: &mut App| {
            cx.init_colors();
            gpui_component::init(cx);

            let tree = build_route_tree();

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
                    cx.new(|cx| Root::new(root_view, window, cx))
                },
            )
            .unwrap();

            cx.activate(true);
        });
}
