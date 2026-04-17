// navi-router/src/navigator.rs
use crate::location::{Location, NavigateOptions};
use crate::state::RouterState;
use gpui::{AnyWindowHandle, App};

/// A handle for programmatic navigation.
#[derive(Clone)]
pub struct Navigator {
    window: AnyWindowHandle,
    base: Option<String>,
}

impl Navigator {
    /// Creates a new `Navigator` for the given window.
    pub fn new(window: AnyWindowHandle) -> Self {
        Self { window, base: None }
    }

    /// Creates a `Navigator` with a base path. Relative navigation will be resolved against this base.
    pub fn from_route(window: AnyWindowHandle, base: impl Into<String>) -> Self {
        Self {
            window,
            base: Some(base.into()),
        }
    }

    /// Navigates to the given path by pushing a new entry onto the history stack.
    pub fn push(&self, path: impl Into<String>, cx: &mut App) {
        let loc = self.to_location(path);
        self.push_location(loc, cx);
    }

    /// Navigates to the given `Location` by pushing a new entry.
    pub fn push_location(&self, loc: Location, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            state.navigate(loc, NavigateOptions::default(), cx);
            cx.refresh_windows();
        });
    }

    /// Replaces the current history entry with the given path.
    pub fn replace(&self, path: impl Into<String>, cx: &mut App) {
        let loc = self.to_location(path);
        self.replace_location(loc, cx);
    }

    /// Replaces the current history entry with the given `Location`.
    pub fn replace_location(&self, loc: Location, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            state.navigate(
                loc,
                NavigateOptions {
                    replace: true,
                    ..Default::default()
                },
                cx,
            );
            cx.refresh_windows();
        });
    }

    /// Navigates back one entry in the history stack.
    pub fn back(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            if state.history.back() {
                cx.refresh_windows();
            }
        });
    }

    /// Navigates forward one entry in the history stack.
    pub fn forward(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            if state.history.forward() {
                cx.refresh_windows();
            }
        });
    }

    /// Moves by `delta` steps in the history stack. Positive = forward, negative = back.
    pub fn go(&self, delta: isize, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            state.history.go(delta);
            cx.refresh_windows();
        });
    }

    /// Returns `true` if back navigation is possible from the current global router state.
    pub fn can_go_back(cx: &App) -> bool {
        RouterState::try_global(cx)
            .map(|state| state.history.can_go_back())
            .unwrap_or(false)
    }

    /// Returns `true` if forward navigation is possible from the current global router state.
    pub fn can_go_forward(cx: &App) -> bool {
        RouterState::try_global(cx)
            .map(|state| state.history.can_go_forward())
            .unwrap_or(false)
    }

    /// Preloads a route without performing navigation. Runs the loader in the background and caches the result.
    pub fn preload(&self, path: impl Into<String>, cx: &mut App) {
        let loc = self.to_location(path);
        RouterState::update(cx, |state, cx| {
            state.preload_location(loc, cx);
        });
    }

    /// Returns the window handle associated with this navigator.
    pub fn window(&self) -> AnyWindowHandle {
        self.window
    }

    /// Resolves a path into a full `Location`. If the path is relative and a `base` is set, it is resolved against it.
    fn to_location(&self, path: impl Into<String>) -> Location {
        let path = path.into();
        let resolved = if path.starts_with('/') {
            path
        } else if let Some(base) = &self.base {
            format!("{}/{}", base.trim_end_matches('/'), path)
        } else {
            path
        };
        Location::new(&resolved)
    }
}
