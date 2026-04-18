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
use navi_devtools::DeepLinkView;
#[cfg(feature = "nexum")]
use navi_router::deep_link;

mod routes;
mod route_tree {
    include!("route_tree.gen.rs");
}
use route_tree::build_route_tree;

struct AppView {
    router_provider: RouterProvider,
    devtools: Entity<DevtoolsState>,
    #[cfg(feature = "nexum")]
    deep_link_view: Entity<DeepLinkView>,
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut base = div()
            .size_full()
            .relative()
            .child(self.router_provider.clone().child(Outlet::new()))
            .child(self.devtools.clone());

        #[cfg(feature = "nexum")]
        {
            base = base.child(self.deep_link_view.clone());
        }

        base.children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

fn main() {
    env_logger::init();
    log::info!("Starting Navi example app with file-based routing");

    let app = gpui_platform::application();
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
                let main_scroll_handle = ScrollHandle::new();

                let router_provider = RouterProvider::new(
                    window_id,
                    window_handle,
                    initial,
                    tree,
                    main_scroll_handle,
                    cx,
                );

                route_tree::register_routes(cx);

                // Initialize deep linking if feature is enabled
                #[cfg(feature = "nexum")]
                {
                    // FIX: GPUI's Application wraps an OS-level singleton. Calling this
                    // inside .run() safely retrieves a handle to the active application.
                    let app_handle = gpui_platform::application();

                    deep_link::init(&app_handle, vec!["naviapp".to_string()], window_handle, cx);
                }

                let query_client = RouterState::global(cx).query_client.clone();
                let devtools = cx.new(|cx| DevtoolsState::new(query_client, cx));

                #[cfg(feature = "nexum")]
                let deep_link_view = cx.new(|_cx| DeepLinkView::new());

                let root_view = cx.new(|_cx| AppView {
                    router_provider,
                    devtools,
                    #[cfg(feature = "nexum")]
                    deep_link_view,
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
