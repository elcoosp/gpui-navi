use crate::RouterState;
use gpui::{
    App, Element, ElementId, Global, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, ScrollHandle, Window, point, px,
};
use std::collections::HashMap;

/// Global storage for scroll positions, keyed by route pathname.
struct ScrollPositions(HashMap<String, f32>);
impl Global for ScrollPositions {}

pub struct ScrollRestoration {
    scroll_handle: ScrollHandle,
}

impl ScrollRestoration {
    pub fn new(scroll_handle: ScrollHandle) -> Self {
        Self { scroll_handle }
    }

    /// Save the current scroll offset for the given path.
    fn save(path: &str, offset: f32, cx: &mut App) {
        let positions = cx.global_mut::<ScrollPositions>();
        positions.0.insert(path.to_string(), offset);
    }

    /// Get a previously saved scroll offset for the given path.
    fn get(path: &str, cx: &App) -> Option<f32> {
        cx.try_global::<ScrollPositions>()
            .and_then(|p| p.0.get(path).copied())
    }
}

impl Element for ScrollRestoration {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = window.request_layout(Default::default(), [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        _window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(router) = RouterState::try_global(cx) {
            let path = router.current_location().pathname.clone();
            let offset = self.scroll_handle.offset().y.as_f32();
            if offset != 0.0 {
                Self::save(&path, offset, cx);
            }
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(router) = RouterState::try_global(cx) {
            let path = router.current_location().pathname.clone();
            if let Some(saved) = Self::get(&path, cx) {
                let scroll_handle = self.scroll_handle.clone();
                let target = point(scroll_handle.offset().x, px(saved));
                window.on_next_frame(move |window, _| {
                    scroll_handle.set_offset(target);
                    window.refresh();
                });
            }
        }
    }
}

impl IntoElement for ScrollRestoration {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}
