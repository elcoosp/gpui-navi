use crate::{Location, NavigateOptions};
use crate::state::RouterState;
use gpui::{AnyWindowHandle, App};

#[derive(Clone)]
pub struct Navigator {
    window: AnyWindowHandle,
    base: Option<String>,
}

impl Navigator {
    pub fn new(window: AnyWindowHandle) -> Self { Self { window, base: None } }
    pub fn from_route(window: AnyWindowHandle, base: impl Into<String>) -> Self { Self { window, base: Some(base.into()) } }
    pub fn push(&self, path: impl Into<String>, cx: &mut App) { let loc = self.to_location(path); self.push_location(loc, cx); }
    pub fn push_location(&self, loc: Location, cx: &mut App) {
        RouterState::update(cx, |state, cx| { state.navigate(loc, NavigateOptions::default(), cx); cx.refresh_windows(); });
    }
    pub fn replace(&self, path: impl Into<String>, cx: &mut App) { let loc = self.to_location(path); self.replace_location(loc, cx); }
    pub fn replace_location(&self, loc: Location, cx: &mut App) {
        RouterState::update(cx, |state, cx| { state.navigate(loc, NavigateOptions { replace: true, ..Default::default() }, cx); cx.refresh_windows(); });
    }
    pub fn back(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| { if state.can_go_back() { state.back(); cx.refresh_windows(); } });
    }
    pub fn forward(&self, cx: &mut App) {
        RouterState::update(cx, |state, cx| { if state.can_go_forward() { state.forward(); cx.refresh_windows(); } });
    }
    pub fn go(&self, delta: isize, cx: &mut App) {
        RouterState::update(cx, |state, cx| { state.go(delta); cx.refresh_windows(); });
    }
    pub fn can_go_back(cx: &App) -> bool { RouterState::global(cx).can_go_back() }
    pub fn can_go_forward(cx: &App) -> bool { RouterState::global(cx).can_go_forward() }
    pub fn preload(&self, path: impl Into<String>, cx: &mut App) {
        let loc = self.to_location(path); RouterState::update(cx, |state, cx| { state.preload_location(loc, cx); });
    }
    pub fn window(&self) -> AnyWindowHandle { self.window }
    fn to_location(&self, path: impl Into<String>) -> Location {
        let path = path.into(); let resolved = if path.starts_with('/') { path } else if let Some(base) = &self.base { format!("{}/{}", base.trim_end_matches('/'), path) } else { path };
        Location::new(&resolved)
    }
}
