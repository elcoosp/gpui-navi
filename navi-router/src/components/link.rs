use crate::{Navigator, RouterState};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, MouseButton, MouseUpEvent, ParentElement,
    RenderOnce, Styled, Window, div,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default)]
pub enum PreloadType {
    Intent,
    Viewport,
    #[default]
    Render,
}

#[derive(IntoElement)]
pub struct Link {
    to: String,
    search: Option<serde_json::Value>,
    hash: Option<String>,
    state: Option<serde_json::Value>,
    replace: bool,
    #[allow(dead_code)]
    reset_scroll: Option<bool>,
    preload: Option<PreloadType>,
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn build_location(&self) -> crate::Location {
        let mut loc = crate::Location::new(&self.to);
        if let Some(search) = &self.search {
            loc.search = search.clone();
        }
        if let Some(hash) = &self.hash {
            loc.hash = hash.clone();
        }
        if let Some(state) = &self.state {
            loc.state = state.clone();
        }
        loc
    }
}

impl ParentElement for Link {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Link {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let to = self.to.clone();
        let replace = self.replace;
        let disabled = self.disabled;
        let search = self.search.clone();
        let hash = self.hash.clone();
        let state = self.state.clone();

        let is_active = RouterState::try_global(cx)
            .map(|router_state: &RouterState| router_state.current_location().pathname == to)
            .unwrap_or(false);

        let navigator = Navigator::new(window.window_handle());

        let mut element = div().cursor_pointer().children(self.children);

        if is_active {
            if let Some(class) = &self.active_class {
                element = element.child(class.clone());
            }
        } else {
            if let Some(class) = &self.inactive_class {
                element = element.child(class.clone());
            }
        }

        element.on_mouse_up(
            MouseButton::Left,
            move |_event: &MouseUpEvent, _window, cx| {
                if !disabled {
                    let mut loc = crate::Location::new(&to);
                    if let Some(s) = &search {
                        loc.search = s.clone();
                    }
                    if let Some(h) = &hash {
                        loc.hash = h.clone();
                    }
                    if let Some(st) = &state {
                        loc.state = st.clone();
                    }
                    if replace {
                        navigator.replace(loc.pathname, cx);
                    } else {
                        navigator.push(loc.pathname, cx);
                    }
                }
            },
        )
    }
}
