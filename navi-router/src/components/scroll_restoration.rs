use crate::RouterState;
use gpui::{
    App, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, ScrollHandle,
    Window, point,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static SCROLL_POSITIONS: Lazy<Mutex<HashMap<String, f32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub struct ScrollRestoration {
    scroll_handle: ScrollHandle,
}

impl ScrollRestoration {
    pub fn new(scroll_handle: ScrollHandle) -> Self {
        Self { scroll_handle }
    }

    fn save(path: &str, offset: f32) {
        SCROLL_POSITIONS.lock().unwrap().insert(path.to_string(), offset);
    }

    fn get(path: &str) -> Option<f32> {
        SCROLL_POSITIONS.lock().unwrap().get(path).copied()
    }
}

impl Element for ScrollRestoration {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
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
        let state = RouterState::try_global(cx);
        if let Some(state) = state {
            let path = state.current_location().pathname.clone();
            let offset_y = self.scroll_handle.offset().y;
            let offset_val = offset_y.as_f32(); // use public method instead of .0
            if offset_val != 0.0 {
                Self::save(&path, offset_val);
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
        let state = RouterState::try_global(cx);
        if let Some(state) = state {
            let path = state.current_location().pathname.clone();
            if let Some(saved_y) = Self::get(&path) {
                let scroll_handle = self.scroll_handle.clone();
                // Use Pixels::from(saved_y) instead of tuple struct constructor
                let saved = point(scroll_handle.offset().x, Pixels::from(saved_y));
                window.on_next_frame(move |window, _| {
                    scroll_handle.set_offset(saved);
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
