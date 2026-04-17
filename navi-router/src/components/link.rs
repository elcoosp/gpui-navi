use crate::{Navigator, RouterState};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, MouseButton, MouseUpEvent, ParentElement,
    RenderOnce, Styled, Window, div, FontWeight,
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
    href: String,
    search: Option<serde_json::Value>,
    hash: Option<String>,
    state: Option<serde_json::Value>,
    replace: bool,
    preload: Option<PreloadType>,
    #[allow(dead_code)]

    #[allow(dead_code)]

    preload_delay: Option<Duration>,
    disabled: bool,
    exact: bool,
    active_style: Option<Box<dyn Fn(gpui::Div) -> gpui::Div>>,
    inactive_style: Option<Box<dyn Fn(gpui::Div) -> gpui::Div>>,
    children: Vec<AnyElement>,
}

impl Link {
    pub fn new(to: impl Into<String>) -> Self {
        Self {
            href: to.into(),
            search: None,
            hash: None,
            state: None,
            replace: false,
            preload: None,
            preload_delay: None,
            disabled: false,
            exact: false,
            active_style: None,
            inactive_style: None,
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

    pub fn exact(mut self) -> Self {
        self.exact = true;
        self
    }

    pub fn active_style(mut self, f: impl Fn(gpui::Div) -> gpui::Div + 'static) -> Self {
        self.active_style = Some(Box::new(f));
        self
    }

    pub fn inactive_style(mut self, f: impl Fn(gpui::Div) -> gpui::Div + 'static) -> Self {
        self.inactive_style = Some(Box::new(f));
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
        &self.href
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl ParentElement for Link {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Link {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let href = self.href.clone();
        let replace = self.replace;
        let disabled = self.disabled;
        let search = self.search.clone();
        let hash = self.hash.clone();
        let state = self.state.clone();
        let exact = self.exact;

        let is_active = RouterState::try_global(cx)
            .map(|router_state: &RouterState| {
                let current = &router_state.current_location().pathname;
                if exact {
                    current == &href
                } else {
                    current.starts_with(&href)
                }
            })
            .unwrap_or(false);

        let navigator = Navigator::new(window.window_handle());

        let mut element = div().cursor_pointer().children(self.children);

        if is_active {
            if let Some(f) = self.active_style {
                element = f(element);
            } else {
                element = element.font_weight(FontWeight::BOLD);
            }
        } else if let Some(f) = self.inactive_style {
            element = f(element);
        }

        element.on_mouse_up(
            MouseButton::Left,
            move |_event: &MouseUpEvent, _window, cx| {
                if !disabled {
                    let mut loc = crate::Location::new(&href);
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
                        navigator.replace_location(loc, cx);
                    } else {
                        navigator.push_location(loc, cx);
                    }
                }
            },
        )
    }
}
