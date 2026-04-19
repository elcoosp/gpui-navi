use gpui::ScrollHandle;
use gpui::prelude::*;
use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;
use navi_devtools::DevtoolsState;
use navi_router::{
    Location, RouterState,
    components::{Outlet, RouterProvider},
};

#[cfg(feature = "nexum")]
use navi_router::deep_link;

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
    log::info!("Starting Navi example app with file-based routing");

    let app = gpui_platform::application();

    // STEP 1: Setup deep links BEFORE app.run() consumes the handle!
    // This perfectly matches your working example.
    #[cfg(feature = "nexum")]
    let deep_link_handle = deep_link::setup(&app, vec!["naviapp".to_string()]);

    let app = app.with_assets(Assets);

    app.run(move |cx: &mut App| {
        cx.init_colors();
        gpui_component::init(cx);

        log::info!("Building route tree from generated code");
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

                let router_provider = RouterProvider::new(
                    window_id,
                    window_handle,
                    initial,
                    tree,
                    cx,
                );

                route_tree::register_routes(cx);

                // STEP 2: Attach the listener INSIDE the window context!
                // This perfectly matches your working example's attach_deep_link call.
                #[cfg(feature = "nexum")]
                deep_link::attach(deep_link_handle, window_handle, cx);

                let query_client = RouterState::global(cx).query_client.clone();

                // Devtools now creates and manages the DeepLinkView internally!
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
        log::info!("Application running");
    });
}
