use crate::state::RouterState;
use crate::{Location, NavOp, NavigateOptions};
use gpui::{AnyWindowHandle, App};

#[derive(Clone)]
pub struct Navigator {
    window: AnyWindowHandle,
    base: Option<String>,
}

impl Navigator {
    #[must_use]
    pub fn new(window: AnyWindowHandle) -> Self {
        Self { window, base: None }
    }
    pub fn from_route(window: AnyWindowHandle, base: impl Into<String>) -> Self {
        Self {
            window,
            base: Some(base.into()),
        }
    }
    pub fn push(&self, path: impl Into<String>, cx: &mut App) {
        let loc = self.to_location(path);
        self.push_location(loc, cx);
    }
    pub fn push_location(&self, loc: Location, cx: &mut App) {
        self.push_location_with(loc, &NavigateOptions::default(), cx);
    }

    /// Push with explicit navigation options (scroll reset, view transition,
    /// blocker override, ...).
    pub fn push_location_with(&self, loc: Location, options: &NavigateOptions, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            state.navigate(loc, options, cx);
            cx.refresh_windows();
        });
    }
    pub fn replace(&self, path: impl Into<String>, cx: &mut App) {
        let loc = self.to_location(path);
        self.replace_location(loc, cx);
    }
    pub fn replace_location(&self, loc: Location, cx: &mut App) {
        self.replace_location_with(loc, &NavigateOptions::default(), cx);
    }

    /// Replace with explicit navigation options.
    pub fn replace_location_with(&self, loc: Location, options: &NavigateOptions, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            state.navigate(
                loc,
                &NavigateOptions {
                    replace: true,
                    ..options.clone()
                },
                cx,
            );
            cx.refresh_windows();
        });
    }

    /// Apply a batch of navigation operations atomically (one re-render).
    pub fn transact(&self, ops: impl IntoIterator<Item = NavOp>, cx: &mut App) {
        let ops: Vec<NavOp> = ops.into_iter().collect();
        RouterState::update(cx, |state, cx| {
            state.transact(&ops, cx);
            cx.refresh_windows();
        });
    }
    pub fn back(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            if state.can_go_back() {
                state.back();
                cx.refresh_windows();
            }
        });
    }
    pub fn forward(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            if state.can_go_forward() {
                state.forward();
                cx.refresh_windows();
            }
        });
    }
    pub fn go(&self, delta: isize, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            state.go(delta);
            cx.refresh_windows();
        });
    }
    pub fn can_go_back(cx: &App) -> bool {
        RouterState::global(cx).can_go_back()
    }
    pub fn can_go_forward(cx: &App) -> bool {
        RouterState::global(cx).can_go_forward()
    }
    pub fn preload(&self, path: impl Into<String>, cx: &mut App) {
        let loc = self.to_location(path);
        RouterState::update(cx, |state, cx| {
            state.preload_location(&loc, cx);
        });
    }
    #[must_use]
    pub fn window(&self) -> AnyWindowHandle {
        self.window
    }
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
