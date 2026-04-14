use gpui::{AnyElement, App, IntoElement};
use std::time::Duration;

/// Type of preloading strategy for links.
#[derive(Clone, Copy, Debug, Default)]
pub enum PreloadType {
    Intent,
    Viewport,
    #[default]
    Render,
}

/// Link component that navigates to a new route.
pub struct Link {
    to: String,
    search: Option<serde_json::Value>,
    hash: Option<String>,
    state: Option<serde_json::Value>,
    replace: bool,
    reset_scroll: Option<bool>,
    preload: Option<PreloadType>,
    preload_delay: Option<Duration>,
    disabled: bool,
    active_class: Option<String>,
    inactive_class: Option<String>,
    children: Vec<AnyElement>,
}

impl Link {
    pub fn new(to: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            search: None,
            hash: None,
            state: None,
            replace: false,
            reset_scroll: None,
            preload: None,
            preload_delay: None,
            disabled: false,
            active_class: None,
            inactive_class: None,
            children: Vec::new(),
        }
    }

    pub fn search(mut self, search: serde_json::Value) -> Self {
        self.search = Some(search);
        self
    }

    pub fn hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(hash.into());
        self
    }

    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn active_class(mut self, class: impl Into<String>) -> Self {
        self.active_class = Some(class.into());
        self
    }

    pub fn inactive_class(mut self, class: impl Into<String>) -> Self {
        self.inactive_class = Some(class.into());
        self
    }

    pub fn preload(mut self, preload: PreloadType) -> Self {
        self.preload = Some(preload);
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn to(&self) -> &str {
        &self.to
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl IntoElement for Link {
    fn into_any_element(self) -> AnyElement {
        gpui::div().into_any_element()
    }
}
