use crate::RouterState;
use gpui::{
    App, Element, ElementId, Global, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, ScrollHandle, Style, Window, point, px,
};
use std::collections::HashMap;

/// Default upper bound on remembered scroll positions. Bounded memory even
/// for apps that wander through thousands of dynamic routes.
pub const DEFAULT_SCROLL_CACHE_CAPACITY: usize = 128;

/// Global storage for scroll positions, keyed by path + search. Entries are
/// capped: inserting beyond the capacity evicts the oldest entry.
struct ScrollPositions {
    map: HashMap<String, f32>,
    order: Vec<String>,
    capacity: usize,
}

impl ScrollPositions {
    fn key(pathname: &str, search: &serde_json::Value) -> String {
        let search_key = canonical_search_key(search);
        if search_key.is_empty() {
            pathname.to_string()
        } else {
            format!("{pathname}?{search_key}")
        }
    }

    fn insert(&mut self, key: String, offset: f32) {
        if !self.map.contains_key(&key) {
            self.order.push(key.clone());
            while self.order.len() > self.capacity {
                let evicted = self.order.remove(0);
                self.map.remove(&evicted);
            }
        }
        self.map.insert(key, offset);
    }

    fn get(&self, key: &str) -> Option<f32> {
        self.map.get(key).copied()
    }
}

impl Default for ScrollPositions {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            order: Vec::new(),
            capacity: DEFAULT_SCROLL_CACHE_CAPACITY,
        }
    }
}

impl Global for ScrollPositions {}

/// Build a stable, insertion-order-independent string for a search value so
/// `?a=1&b=2` and `?b=2&a=1` share one scroll entry. Nested JSON is handled
/// by serializing with object keys sorted.
fn canonical_search_key(search: &serde_json::Value) -> String {
    match search {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => {
            let mut pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{k}={}", canonical_search_key(v)))
                .collect();
            pairs.sort();
            pairs.join("&")
        }
        other => other.to_string(),
    }
}

pub struct ScrollRestoration {
    scroll_handle: ScrollHandle,
}

impl ScrollRestoration {
    #[must_use]
    pub fn new(scroll_handle: ScrollHandle) -> Self {
        Self { scroll_handle }
    }

    /// Bound how many locations remember their scroll offset (oldest evicted
    /// first). Defaults to [`DEFAULT_SCROLL_CACHE_CAPACITY`].
    pub fn set_capacity(capacity: usize, cx: &mut App) {
        cx.global_mut::<ScrollPositions>().capacity = capacity;
    }

    /// Save the current scroll offset for the given path.
    fn save(path: &str, offset: f32, cx: &mut App) {
        cx.global_mut::<ScrollPositions>()
            .insert(path.to_string(), offset);
    }

    /// Get a previously saved scroll offset for the given path.
    fn get(path: &str, cx: &App) -> Option<f32> {
        cx.try_global::<ScrollPositions>().and_then(|p| p.get(path))
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
        let layout_id = window.request_layout(Style::default(), [], cx);
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
        if let Some(router) = RouterState::try_global(cx)
            && router.scroll_restoration
        {
            let loc = router.current_location();
            let key = ScrollPositions::key(&loc.pathname, &loc.search);
            let offset = self.scroll_handle.offset().y.as_f32();
            if offset != 0.0 {
                Self::save(&key, offset, cx);
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
        if RouterState::try_global(cx).is_none() {
            return;
        }
        // Restoration is opt-out (`scroll_restoration: false`) and skipped
        // once after a navigation that asked for a scroll reset.
        let (enabled, reset) = RouterState::update(cx, |state, _| {
            (state.scroll_restoration, state.take_scroll_reset())
        });
        if !enabled || reset {
            return;
        }
        let loc = RouterState::global(cx).current_location();
        let key = ScrollPositions::key(&loc.pathname, &loc.search);
        if let Some(saved) = Self::get(&key, cx) {
            let scroll_handle = self.scroll_handle.clone();
            let target = point(scroll_handle.offset().x, px(saved));
            window.on_next_frame(move |window, _| {
                scroll_handle.set_offset(target);
                window.refresh();
            });
        }
    }
}

impl IntoElement for ScrollRestoration {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_search_key_ignores_object_order() {
        let a = canonical_search_key(&json!({"a": "1", "b": "2"}));
        let b = canonical_search_key(&json!({"b": "2", "a": "1"}));
        assert_eq!(a, b);
        assert_eq!(a, "a=1&b=2");
    }

    #[test]
    fn test_search_key_handles_scalars_and_null() {
        assert_eq!(canonical_search_key(&json!(null)), "");
        assert_eq!(canonical_search_key(&json!("q=raw")), "q=raw");
        assert_eq!(canonical_search_key(&json!(42)), "42");
        assert_eq!(
            canonical_search_key(&json!({"page": 2, "tag": ["x", "y"]})),
            "page=2&tag=[\"x\",\"y\"]"
        );
    }

    #[test]
    fn test_position_key_distinguishes_search() {
        assert_eq!(
            ScrollPositions::key("/items", &json!({"page": 1})),
            ScrollPositions::key("/items", &json!({"page": 1}))
        );
        assert_ne!(
            ScrollPositions::key("/items", &json!({"page": 1})),
            ScrollPositions::key("/items", &json!({"page": 2}))
        );
        assert_eq!(ScrollPositions::key("/items", &json!(null)), "/items");
    }

    #[test]
    fn test_capacity_evicts_oldest() {
        let mut store = ScrollPositions {
            capacity: 2,
            ..ScrollPositions::default()
        };
        store.insert("/a".to_string(), 1.0);
        store.insert("/b".to_string(), 2.0);
        assert_eq!(store.get("/a"), Some(1.0));
        // Re-saving an existing key must not evict anything.
        store.insert("/b".to_string(), 20.0);
        store.insert("/c".to_string(), 3.0);
        // "/a" is oldest and evicted, "/b" kept because it was refreshed.
        assert_eq!(store.get("/a"), None);
        assert_eq!(store.get("/b"), Some(20.0));
        assert_eq!(store.get("/c"), Some(3.0));
    }
}
