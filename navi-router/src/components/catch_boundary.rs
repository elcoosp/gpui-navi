use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Window, div};
use std::panic::{self, AssertUnwindSafe};

#[derive(Default)]
pub struct CatchBoundary {
    error_component: Option<AnyElement>,
    children: Vec<AnyElement>,
}

impl CatchBoundary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error_component(mut self, component: impl IntoElement) -> Self {
        self.error_component = Some(component.into_any_element());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl ParentElement for CatchBoundary {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for CatchBoundary {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            div().children(self.children).into_any_element()
        }));

        match result {
            Ok(element) => element,
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                log::error!("Route component panicked: {}", msg);

                self.error_component.unwrap_or_else(|| {
                    div()
                        .child("Something went wrong")
                        .child(msg)
                        .into_any_element()
                })
            }
        }
    }
}
