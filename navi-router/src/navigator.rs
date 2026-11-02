use crate::state::RouterHandle;
use crate::{Location, NavOp, NavigateOptions};

/// Imperative navigation API.
///
/// A `Navigator` is a cheap handle to the thread-current (or explicitly
/// provided) [`RouterHandle`]; all methods mutate the shared state and
/// request a repaint through the router host.
#[derive(Clone)]
pub struct Navigator {
    handle: RouterHandle,
}

impl Navigator {
    /// A navigator over the thread-current router.
    ///
    /// # Panics
    /// Panics when no router is installed on this thread.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handle: RouterHandle::current(),
        }
    }

    /// A navigator over the thread-current router with a path base
    /// (relative pushes resolve against `base`).
    ///
    /// # Panics
    /// Panics when no router is installed on this thread.
    #[must_use]
    pub fn from_route(base: impl Into<String>) -> Self {
        let handle = RouterHandle::current();
        handle.write().current_base = Some(base.into());
        Self { handle }
    }

    /// A navigator over an explicit router handle (used inside async
    /// continuations where the thread-current slot may have moved on).
    #[must_use]
    pub fn from_handle(handle: &RouterHandle) -> Self {
        Self {
            handle: handle.clone(),
        }
    }

    /// A navigator over an explicit router handle with a path base.
    #[must_use]
    pub fn from_handle_route(handle: &RouterHandle, base: impl Into<String>) -> Self {
        let navigator = Self::from_handle(handle);
        handle.write().current_base = Some(base.into());
        navigator
    }

    /// The backing router handle.
    #[must_use]
    pub fn handle(&self) -> RouterHandle {
        self.handle.clone()
    }

    pub fn push(&self, path: impl Into<String>) {
        let loc = self.to_location(path);
        self.push_location(loc);
    }

    pub fn push_location(&self, loc: Location) {
        self.push_location_with(loc, &NavigateOptions::default());
    }

    /// Push with explicit navigation options (scroll reset, view transition,
    /// blocker override, ...).
    pub fn push_location_with(&self, loc: Location, options: &NavigateOptions) {
        self.handle.update(|state| {
            state.navigate(loc, options);
        });
        self.handle.refresh();
    }
    pub fn replace(&self, path: impl Into<String>) {
        let loc = self.to_location(path);
        self.replace_location(loc);
    }
    pub fn replace_location(&self, loc: Location) {
        self.replace_location_with(loc, &NavigateOptions::default());
    }

    /// Replace with explicit navigation options.
    pub fn replace_location_with(&self, loc: Location, options: &NavigateOptions) {
        self.handle.update(|state| {
            state.navigate(
                loc,
                &NavigateOptions {
                    replace: true,
                    ..options.clone()
                },
            );
        });
        self.handle.refresh();
    }

    /// Apply a batch of navigation operations atomically (one re-render).
    pub fn transact(&self, ops: impl IntoIterator<Item = NavOp>) {
        let ops: Vec<NavOp> = ops.into_iter().collect();
        self.handle.update(|state| {
            state.transact(&ops);
        });
        self.handle.refresh();
    }
    pub fn back(&self) {
        self.handle.update(|state| {
            if state.can_go_back() {
                state.back();
            }
        });
        self.handle.refresh();
    }
    pub fn forward(&self) {
        self.handle.update(|state| {
            if state.can_go_forward() {
                state.forward();
            }
        });
        self.handle.refresh();
    }
    pub fn go(&self, delta: isize) {
        self.handle.update(|state| {
            state.go(delta);
        });
        self.handle.refresh();
    }
    #[must_use]
    pub fn can_go_back() -> bool {
        RouterHandle::current().read().can_go_back()
    }
    #[must_use]
    pub fn can_go_forward() -> bool {
        RouterHandle::current().read().can_go_forward()
    }
    pub fn preload(&self, path: impl Into<String>) {
        let loc = self.to_location(path);
        self.handle.update(|state| {
            state.preload_location(&loc);
        });
    }
    fn to_location(&self, path: impl Into<String>) -> Location {
        let path = path.into();
        let resolved = if path.starts_with('/') {
            path
        } else {
            let base = self.handle.read().current_base.clone();
            match base {
                Some(base) => format!("{}/{}", base.trim_end_matches('/'), path),
                None => path,
            }
        };
        Location::new(&resolved)
    }
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new()
    }
}
