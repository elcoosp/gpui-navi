use gpui::{
    App, AppContext, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use gpui_kit::assets::Assets;
use navi_core::WindowKey;
use navi_devtools::{DevtoolsModel, SharedModel};
use navi_router::{
    Location, NotFoundMode, RouterHost, RouterOptions, RouterProvider, components::Outlet,
};
use quoin_ui_gpui::{NodeElement, NodeRenderer};
use send_wrapper::SendWrapper;
use std::cell::RefCell;
use std::rc::Rc;

mod route_tree {
    include!("route_tree.gen.rs");
}
pub mod tour;

use route_tree::build_route_tree;

/// The application root: the framework-agnostic router tree plus the
/// devtools panel, both rendered through quoin's GPUI adapter.
struct AppView {
    provider: RouterProvider,
    devtools: SharedModel,
    renderer: NodeRenderer,
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Bind the devtools keybindings once per app.
        self.renderer
            .ensure_keybindings(cx, &self.devtools.borrow().keybindings());

        // Router tree (framework-agnostic Node) around the outlet. The
        // outlet tree is built lazily by the provider (after it resets the
        // outlet depth), mirroring the classic provider render order.
        let router_node = self
            .provider
            .clone()
            .child(|| Outlet::new().build())
            .build();

        // Devtools panel (framework-agnostic Node) through the renderer.
        let viewport_size = window.viewport_size();
        let viewport = (viewport_size.width.as_f32(), viewport_size.height.as_f32());
        let focus = self.devtools.borrow_mut().take_focus_request();
        let inputs = self.renderer.input_values(cx);
        let devtools_node = DevtoolsModel::render(&self.devtools, &inputs, viewport);
        let devtools_element = self
            .renderer
            .render(&devtools_node, focus.as_deref(), window, cx);

        // Dialog, sheet and notification layers are mounted by the kit's
        // Root itself in the post-restructure stack; the app view renders
        // only its own content.
        div()
            .size_full()
            .relative()
            .child(NodeElement(router_node))
            .child(devtools_element)
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
                    let window_key = WindowKey(window.window_handle().window_id().as_u64());

                    // Router host seam: UI-thread spawn + repaint through a
                    // clonable AsyncApp handle. Both hooks run on the main
                    // thread (GPUI's threading model), so the handle is
                    // wrapped for the `Rc`-based seam via `SendWrapper`.
                    let async_cx = SendWrapper::new(cx.to_async());
                    let async_cx_spawn = async_cx.clone();
                    let host = RouterHost {
                        refresh: Rc::new(move || {
                            let cx = async_cx.clone();
                            cx.spawn(async move |cx| {
                                cx.update(gpui::App::refresh_windows);
                            })
                            .detach();
                        }),
                        spawn: Rc::new(move |future| {
                            let cx = async_cx_spawn.clone();
                            cx.spawn(async move |_| {
                                future.await;
                            })
                            .detach();
                        }),
                    };

                    let initial = Location::new("/");
                    let provider = RouterProvider::install_with_options(
                        initial,
                        window_key,
                        tree,
                        RouterOptions {
                            default_pending_ms: 500,
                            default_pending_min_ms: 200,
                            not_found_mode: NotFoundMode::Fuzzy,
                        },
                        host.clone(),
                    );

                    route_tree::register_routes();

                    let handle = provider.handle();
                    let query_client = handle.read().query_client.clone();

                    let devtools: SharedModel = Rc::new(RefCell::new(DevtoolsModel::new(
                        query_client,
                        Rc::new(move || (host.refresh)()),
                    )));

                    let root_view = cx.new(|_cx| AppView {
                        provider: RouterProvider::from_handle(&handle),
                        devtools: devtools.clone(),
                        renderer: NodeRenderer::new(),
                    });

                    handle.update(|state| {
                        state.set_root_view(root_view.entity_id().as_u64());
                    });

                    // Screenshot tour (see `tour.rs`): active only when the
                    // environment asks for it, so normal runs are unaffected.
                    if let Ok(dir) = std::env::var("NAVI_TOUR_DIR") {
                        cx.set_global(tour::TourGlobal {
                            devtools: devtools.clone(),
                        });
                        let dir = dir.clone();
                        cx.spawn(move |cx: &mut gpui::AsyncApp| {
                            let dir = dir.clone();
                            let cx = cx.clone();
                            async move { tour::run_tour(cx, dir.into()).await }
                        })
                        .detach();
                    }

                    root_view
                },
            )
            .unwrap();

            cx.activate(true);
        });
}
