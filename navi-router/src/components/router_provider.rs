use crate::{Location, RouteTree, RouterState};
use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Window, WindowId, div};
use navi_core::context;
use std::rc::Rc;

#[derive(IntoElement, Clone)]
pub struct RouterProvider {
    window_id: WindowId,
    initial_location: Location,
    route_tree: Rc<RouteTree>,
}

impl RouterProvider {
    pub fn new(
        window_id: WindowId,
        initial_location: Location,
        route_tree: RouteTree,
        cx: &mut App,
    ) -> Self {
        context::init_window(window_id);
        let route_tree = Rc::new(route_tree);
        let state = RouterState::new(initial_location.clone(), window_id, route_tree.clone());
        cx.set_global(state);
        Self {
            window_id,
            initial_location,
            route_tree,
        }
    }

    pub fn child(self, child: impl IntoElement) -> RouterProviderWithChildren {
        RouterProviderWithChildren {
            provider: self,
            children: vec![child.into_any_element()],
        }
    }
}

impl RenderOnce for RouterProvider {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
    }
}

#[derive(IntoElement)]
pub struct RouterProviderWithChildren {
    provider: RouterProvider,
    children: Vec<AnyElement>,
}

impl ParentElement for RouterProviderWithChildren {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for RouterProviderWithChildren {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Clone values before moving self.provider to avoid partial move
        let initial_location = self.provider.initial_location.clone();
        let window_id = self.provider.window_id;
        let route_tree = self.provider.route_tree.clone();

        // Re-initialize global state if missing
        if RouterState::try_global(cx).is_none() {
            let state = RouterState::new(initial_location, window_id, route_tree);
            cx.set_global(state);
        }

        div().child(self.provider).children(self.children)
    }
}
