//! Root router provider: installs the router and wraps the app tree.

use crate::state::{RouterHandle, RouterOptions};
use crate::{Location, RouteTree};
use navi_core::WindowKey;
use quoin_ui::Node;
use std::sync::Arc;

/// Installs the router for the current thread and renders children.
///
/// `install` performs the classic `RouterProvider::new` side effects
/// (context-tree init, event log, state creation); the builder wraps the
/// app tree in a full-size container and re-installs the state if the
/// thread-current slot was lost (parity with the classic lazy re-init).
#[derive(Clone)]
pub struct RouterProvider {
    handle: RouterHandle,
    initial_location: Option<Location>,
    route_tree: Arc<RouteTree>,
    window_key: WindowKey,
    host: crate::state::RouterHost,
    options: RouterOptions,
}

impl RouterProvider {
    /// Installs a router with the given options and returns the provider.
    #[must_use]
    pub fn install_with_options(
        initial_location: Location,
        window_key: WindowKey,
        route_tree: RouteTree,
        options: RouterOptions,
        host: crate::state::RouterHost,
    ) -> Self {
        crate::event_bus::init_event_log();
        log::info!("RouterProvider::install: initializing context for window {window_key:?}");
        navi_core::context::init_window(window_key);
        let route_tree = Arc::new(route_tree);
        let handle = RouterHandle::new_with_options(
            initial_location.clone(),
            window_key,
            route_tree.clone(),
            options.clone(),
            host.clone(),
        );
        handle.install_current();
        log::info!("RouterProvider installed successfully");
        Self {
            handle,
            initial_location: Some(initial_location),
            route_tree,
            window_key,
            host,
            options,
        }
    }

    /// A provider over an already-installed handle (used by the app view
    /// across frames; lazy re-init pulls the snapshot from the state).
    #[must_use]
    pub fn from_handle(handle: &RouterHandle) -> Self {
        let state = handle.read();
        Self {
            handle: handle.clone(),
            initial_location: None,
            route_tree: state.route_tree.clone(),
            window_key: state.window_key,
            host: state.host.clone(),
            options: RouterOptions {
                default_pending_ms: state.default_pending_ms,
                default_pending_min_ms: state.default_pending_min_ms,
                not_found_mode: state.not_found_mode.clone(),
            },
        }
    }

    /// Installs a router with default options.
    #[must_use]
    pub fn install(
        initial_location: Location,
        window_key: WindowKey,
        route_tree: RouteTree,
        host: crate::state::RouterHost,
    ) -> Self {
        Self::install_with_options(
            initial_location,
            window_key,
            route_tree,
            RouterOptions::default(),
            host,
        )
    }

    /// The installed router handle.
    #[must_use]
    pub fn handle(&self) -> RouterHandle {
        self.handle.clone()
    }

    /// Wraps children under the provider.
    ///
    /// The child is a constructor, not an already-built tree: building the
    /// outlet tree must happen *after* the provider resets the outlet depth
    /// (in [`RouterProviderWithChildren::build`]), mirroring the classic
    /// provider that rendered its children inside its own render pass.
    #[must_use]
    pub fn child(self, child: impl Fn() -> Node + 'static) -> RouterProviderWithChildren {
        RouterProviderWithChildren {
            provider: self,
            children: vec![Box::new(child)],
        }
    }

    /// Wraps several children under the provider.
    #[must_use]
    pub fn children(
        self,
        children: impl IntoIterator<Item = Box<dyn Fn() -> Node>>,
    ) -> RouterProviderWithChildren {
        RouterProviderWithChildren {
            provider: self,
            children: children.into_iter().collect(),
        }
    }
}

/// The provider with its child constructors; building resets the outlet
/// depth first and then constructs the child trees.
pub struct RouterProviderWithChildren {
    provider: RouterProvider,
    children: Vec<Box<dyn Fn() -> Node>>,
}

impl RouterProviderWithChildren {
    /// Builds the provider's node tree.
    #[must_use]
    pub fn build(self) -> Node {
        if RouterHandle::try_current().is_none() {
            match &self.provider.initial_location {
                Some(initial) => {
                    log::warn!(
                        "RouterProviderWithChildren: current state missing, re-initializing"
                    );
                    let handle = RouterHandle::new_with_options(
                        initial.clone(),
                        self.provider.window_key,
                        self.provider.route_tree.clone(),
                        self.provider.options.clone(),
                        self.provider.host.clone(),
                    );
                    handle.install_current();
                }
                None => {
                    log::warn!(
                        "RouterProviderWithChildren: current state missing and no snapshot to re-install"
                    );
                }
            }
        }
        // Provide initial depth 0 for the root outlet. This must precede
        // the construction of the child trees so the root outlet reads
        // depth 0 on every render pass (the classic provider provided
        // depth 0 inside its render, before its children rendered).
        navi_core::context::provide(self.provider.window_key, super::outlet::OutletDepth(0));
        let children = self
            .children
            .iter()
            .map(|child| child())
            .collect::<Vec<_>>();
        Node::div().children(children).build()
    }
}
