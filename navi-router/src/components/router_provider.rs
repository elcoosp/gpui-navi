// navi-router/src/components/router_provider.rs
use crate::components::outlet::OutletDepth;
use crate::{Location, RouteTree, RouterState};
use gpui::{
    AnyElement, AnyWindowHandle, App, IntoElement, ParentElement, RenderOnce, Window, WindowId, div,
};
use navi_core::context;
use std::rc::Rc;

#[derive(IntoElement, Clone)]
pub struct RouterProvider {
    window_id: WindowId,
    window_handle: AnyWindowHandle,
    initial_location: Location,
    route_tree: Rc<RouteTree>,
}

impl RouterProvider {
    pub fn new(
        window_id: WindowId,
        window_handle: AnyWindowHandle,
        initial_location: Location,
        route_tree: RouteTree,
        cx: &mut App,
    ) -> Self {
        crate::event_bus::init_event_log(cx);
        log::info!(
            "RouterProvider::new: initializing context for window {:?}",
            window_id
        );
        context::init_window(window_id);
        let route_tree = Rc::new(route_tree);
        let state = RouterState::new(
            initial_location.clone(),
            window_id,
            window_handle,
            route_tree.clone(),
        );
        cx.set_global(state);
        log::info!("RouterProvider created successfully");
        Self {
            window_id,
            window_handle,
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
        if RouterState::try_global(cx).is_none() {
            log::warn!("RouterProviderWithChildren: global state missing, re-initializing");
            let state = RouterState::new(
                self.provider.initial_location.clone(),
                self.provider.window_id,
                self.provider.window_handle,
                self.provider.route_tree.clone(),
            );
            cx.set_global(state);
        }
        // Provide initial depth 0 for the root outlet
        cx.provide(OutletDepth(0));
        div().child(self.provider).children(self.children)
    }
}
