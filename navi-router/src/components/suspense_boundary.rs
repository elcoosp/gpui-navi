use crate::RouterState;
use gpui::{AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, Window, div};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(IntoElement)]
pub struct SuspenseBoundary {
    pending_component: Option<AnyElement>,
    pending_ms: u64,
    pending_min_ms: u64,
    children: Vec<AnyElement>,
}

impl SuspenseBoundary {
    pub fn new() -> Self {
        Self {
            pending_component: None,
            pending_ms: 1000,
            pending_min_ms: 500,
            children: Vec::new(),
        }
    }

    pub fn pending_component(mut self, component: impl IntoElement) -> Self {
        self.pending_component = Some(component.into_any_element());
        self
    }

    pub fn pending_ms(mut self, ms: u64) -> Self {
        self.pending_ms = ms;
        self
    }

    pub fn pending_min_ms(mut self, ms: u64) -> Self {
        self.pending_min_ms = ms;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl Default for SuspenseBoundary {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for SuspenseBoundary {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

#[derive(Clone)]
struct LoadingState {
    start: Instant,
    shown: Rc<RefCell<bool>>,
}

impl RenderOnce for SuspenseBoundary {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = RouterState::try_global(cx);
        let is_loading = state.map(|s| s.is_loading()).unwrap_or(false);

        if !is_loading {
            return div().children(self.children).into_any_element();
        }

        let element_id = ElementId::Name("suspense-boundary".into());
        window.with_global_id(element_id, |global_id, window| {
            window.with_element_state::<LoadingState, _>(global_id, |loading_state, window| {
                let loading_state = loading_state.unwrap_or_else(|| LoadingState {
                    start: Instant::now(),
                    shown: Rc::new(RefCell::new(false)),
                });

                let elapsed = loading_state.start.elapsed();
                let should_show = elapsed >= Duration::from_millis(self.pending_ms);

                if should_show && !*loading_state.shown.borrow() {
                    *loading_state.shown.borrow_mut() = true;
                }

                // Keep polling until shown
                if !*loading_state.shown.borrow() {
                    window.request_animation_frame();
                }

                let element = if *loading_state.shown.borrow() {
                    self.pending_component
                        .unwrap_or_else(|| div().child("Loading...").into_any_element())
                } else {
                    div().into_any_element()
                };

                (element, loading_state)
            })
        })
    }
}
