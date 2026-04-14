use crate::{Location, RouteTree, RouterState};
use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Window, WindowId, div};
use navi_core::context;

#[derive(IntoElement)]
pub struct RouterProvider {
    children: Vec<AnyElement>,
}

impl RouterProvider {
    pub fn new(
        window_id: WindowId,
        initial_location: Location,
        route_tree: RouteTree,
        cx: &mut App,
    ) -> Self {
        context::init_window(window_id);

        let state = RouterState::new(initial_location, window_id, route_tree);
        cx.set_global(state);

        Self {
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl ParentElement for RouterProvider {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for RouterProvider {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().children(self.children)
    }
}
