//! Per-framework host seams: the cross-thread refresh callback the task
//! wakers use, plus router installation.
//!
//! The router's spawned completions call `RouterHost::refresh` from the UI
//! thread (futures are polled during render), so an `Rc` closure is fine
//! there. Timer wakes, however, arrive from quoin's background timer
//! thread (`futures_timer::Delay`), so the driver's waker refresh must be
//! `Send + Sync` — each framework provides that differently:
//!
//! - **gpui** — entities are main-thread-bound; the refresh hops to the
//!   main thread through an `AsyncApp` (`refresh_windows`).
//! - **dioxus / leptos / xilem / floem** — quoin signals for those
//!   adapters are `Send + Sync`; writing the shell's tick signal from any
//!   thread schedules the re-render through the adapter's own mechanism
//!   (Dioxus reactor notify, Leptos effects, the Xilem rebuild pump, the
//!   Floem reactive bridge).

use std::rc::Rc;
use std::sync::Arc;

use navi_core::WindowKey;
use navi_router::components::RouterProvider;
use navi_router::{Location, NotFoundMode, RouterHandle, RouterHost, RouterOptions};

use crate::driver::LocalDriver;
use crate::route_tree;

/// Cross-thread refresh: wakes the UI thread so the shell re-renders.
pub type RefreshAny = Arc<dyn Fn() + Send + Sync>;

/// Everything a host needs to run the demo router.
pub struct DemoRouter {
    /// Handle for shell reads/navigations.
    pub handle: RouterHandle,
    /// The driver the shell polls each render.
    pub driver: Rc<LocalDriver>,
}

/// Installs the demo router for `window_key`.
///
/// `refresh_any` must be safe to call from any thread (see the module
/// docs); the UI-thread `RouterHost::refresh` seam simply forwards to it.
pub fn install(refresh_any: RefreshAny, window_key: WindowKey) -> DemoRouter {
    let driver = LocalDriver::new();
    crate::driver::set_thread_driver(driver.clone());

    let refresh_for_host = refresh_any.clone();
    let spawn_driver = driver.clone();
    let host = RouterHost {
        refresh: Rc::new(move || refresh_for_host()),
        spawn: Rc::new(move |future| spawn_driver.spawn(future)),
    };

    crate::driver::poll_router_tasks();
    let tree = route_tree::build_route_tree();
    let provider = RouterProvider::install_with_options(
        Location::new("/"),
        window_key,
        tree,
        RouterOptions {
            default_pending_ms: 500,
            default_pending_min_ms: 200,
            not_found_mode: NotFoundMode::Fuzzy,
        },
        host,
    );
    route_tree::register_routes();

    // Kick the initial loader resolution for the entry route.
    provider.handle().refresh();
    DemoRouter {
        handle: provider.handle(),
        driver,
    }
}
