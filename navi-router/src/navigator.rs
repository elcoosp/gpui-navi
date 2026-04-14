use gpui::{App, Global, WindowId};
use navi_core::context;

use crate::location::{Location, NavigateOptions};
use crate::state::RouterState;

/// Navigator API for programmatic navigation.
#[derive(Clone)]
pub struct Navigator {
    window_id: WindowId,
    base: Option<String>,
}

impl Global for Navigator {}

impl Navigator {
    pub fn new(window_id: WindowId) -> Self {
        Self {
            window_id,
            base: None,
        }
    }

    pub fn from_route(window_id: WindowId, base: impl Into<String>) -> Self {
        Self {
            window_id,
            base: Some(base.into()),
        }
    }

    /// Navigate to a new path by pushing onto the history stack.
    pub fn push(&self, path: impl Into<String>, _cx: &mut App) {
        let _loc = Location::new(&self.resolve_path(path));
        // In a real implementation, this would access RouterState and call navigate
    }

    /// Navigate to a new path by replacing the current history entry.
    pub fn replace(&self, path: impl Into<String>, _cx: &mut App) {
        let _loc = Location::new(&self.resolve_path(path));
    }

    /// Go back in history.
    pub fn back(&self, _cx: &mut App) {
        // Access RouterState and call history.back()
    }

    /// Go forward in history.
    pub fn forward(&self, _cx: &mut App) {
        // Access RouterState and call history.forward()
    }

    /// Go to a specific position in history.
    pub fn go(&self, _delta: isize, _cx: &mut App) {
        // Access RouterState and call history.go()
    }

    /// Check if we can go back in history.
    pub fn can_go_back(_cx: &App) -> bool {
        false
    }

    /// Resolve a possibly relative path against the base.
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

    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
}
