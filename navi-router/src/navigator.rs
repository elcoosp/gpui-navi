use crate::location::{Location, NavigateOptions};
use crate::state::RouterState;
use gpui::{AnyWindowHandle, App};

#[derive(Clone)]
pub struct Navigator {
    window: AnyWindowHandle,
    base: Option<String>,
}

impl Navigator {
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
        let loc = Location::new(&self.resolve_path(path));
        RouterState::update(cx, |state, cx| {
            state.navigate(loc, NavigateOptions::default());
            cx.refresh_windows();
        });
    }

    pub fn replace(&self, path: impl Into<String>, cx: &mut App) {
        let loc = Location::new(&self.resolve_path(path));
        RouterState::update(cx, |state, cx| {
            state.navigate(
                loc,
                NavigateOptions {
                    replace: true,
                    ..Default::default()
                },
            );
            cx.refresh_windows();
        });
    }

    pub fn back(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            if state.history.back() {
                cx.refresh_windows();
            }
        });
    }

    pub fn forward(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            if state.history.forward() {
                cx.refresh_windows();
            }
        });
    }

    pub fn go(&self, delta: isize, cx: &mut App) {
        RouterState::update(cx, |state, cx| {
            state.history.go(delta);
            cx.refresh_windows();
        });
    }

    pub fn can_go_back(cx: &App) -> bool {
        RouterState::try_global(cx)
            .map(|state| state.history.can_go_back())
            .unwrap_or(false)
    }

    fn resolve_path(&self, path: impl Into<String>) -> String {
        let path = path.into();
        if path.starts_with('/') {
            path
        } else if let Some(base) = &self.base {
            format!("{}/{}", base.trim_end_matches('/'), path)
        } else {
            path
        }
    }

    pub fn window(&self) -> AnyWindowHandle {
        self.window
    }
}
