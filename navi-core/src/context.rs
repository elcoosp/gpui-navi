use gpui::WindowId;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;

/// A single layer in the context tree, holding type-keyed values.
pub struct ContextLayer {
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl ContextLayer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<T: 'static>(&mut self, val: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(val));
    }

    #[must_use]
    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
            .cloned()
    }
}

impl Default for ContextLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// A tree of context layers supporting nested scopes for nested routes.
/// Each layer can hold type-keyed values, and consumers search from the
/// topmost (most recent) layer downward.
pub struct ContextTree {
    layers: Vec<ContextLayer>,
    subscriptions: HashMap<TypeId, Vec<Box<dyn Fn() + 'static>>>,
}

impl ContextTree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: vec![ContextLayer::new()],
            subscriptions: HashMap::new(),
        }
    }

    /// Provide a value to the current (topmost) layer.
    pub fn provide<T: 'static>(&mut self, val: T) {
        if let Some(layer) = self.layers.last_mut() {
            layer.insert(val);
        }
        if let Some(subs) = self.subscriptions.get(&TypeId::of::<T>()) {
            for sub in subs {
                sub();
            }
        }
    }

    /// Consume a value by searching layers from top to bottom.
    #[must_use]
    pub fn consume<T: Clone + 'static>(&self) -> Option<T> {
        for layer in self.layers.iter().rev() {
            if let Some(v) = layer.get::<T>() {
                return Some(v);
            }
        }
        None
    }

    /// Push a new context layer (for nested routes).
    pub fn push_layer(&mut self) {
        self.layers.push(ContextLayer::new());
    }

    /// Pop the topmost context layer.
    pub fn pop_layer(&mut self) {
        if self.layers.len() > 1 {
            self.layers.pop();
        }
    }

    /// Subscribe to changes for a specific type.
    pub fn subscribe<T: 'static>(&mut self, f: impl Fn() + 'static) {
        self.subscriptions
            .entry(TypeId::of::<T>())
            .or_default()
            .push(Box::new(f));
    }
}

impl Default for ContextTree {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static WINDOW_CONTEXTS: RefCell<HashMap<WindowId, ContextTree>> = RefCell::new(HashMap::new());
}

/// Initialize a context tree for a window.
pub fn init_window(window_id: WindowId) {
    WINDOW_CONTEXTS.with(|c| {
        c.borrow_mut().insert(window_id, ContextTree::new());
    });
}

/// Destroy the context tree for a window.
pub fn destroy_window(window_id: WindowId) {
    WINDOW_CONTEXTS.with(|c| {
        c.borrow_mut().remove(&window_id);
    });
}

/// Provide a value to the context tree of a specific window.
pub fn provide<T: 'static>(window_id: WindowId, val: T) {
    WINDOW_CONTEXTS.with(|c| {
        if let Some(ctx) = c.borrow_mut().get_mut(&window_id) {
            ctx.provide(val);
        }
    });
}

/// Consume a value from the context tree of a specific window.
#[must_use]
pub fn consume<T: Clone + 'static>(window_id: WindowId) -> Option<T> {
    WINDOW_CONTEXTS.with(|c| c.borrow().get(&window_id)?.consume::<T>())
}

/// Push a new context layer for a window.
pub fn push_layer(window_id: WindowId) {
    WINDOW_CONTEXTS.with(|c| {
        if let Some(ctx) = c.borrow_mut().get_mut(&window_id) {
            ctx.push_layer();
        }
    });
}

/// Pop the topmost context layer for a window.
pub fn pop_layer(window_id: WindowId) {
    WINDOW_CONTEXTS.with(|c| {
        if let Some(ctx) = c.borrow_mut().get_mut(&window_id) {
            ctx.pop_layer();
        }
    });
}

/// Subscribe to changes for a specific type in a window's context.
pub fn subscribe<T: 'static>(window_id: WindowId, f: impl Fn() + 'static) {
    WINDOW_CONTEXTS.with(|c| {
        if let Some(ctx) = c.borrow_mut().get_mut(&window_id) {
            ctx.subscribe::<T>(f);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_layer_insert_and_get() {
        let mut layer = ContextLayer::new();
        layer.insert("value".to_string());
        assert_eq!(layer.get::<String>(), Some("value".to_string()));
        assert!(layer.get::<u8>().is_none());
    }

    #[test]
    fn test_layer_overwrite() {
        let mut layer = ContextLayer::new();
        layer.insert(1_u8);
        layer.insert(2_u8);
        assert_eq!(layer.get::<u8>(), Some(2));
    }

    #[test]
    fn test_layer_default_impl() {
        let layer = ContextLayer::default();
        assert!(layer.get::<String>().is_none());
    }

    #[test]
    fn test_tree_consume_searches_topmost_first() {
        let mut tree = ContextTree::new();
        tree.provide("outer".to_string());
        tree.push_layer();
        tree.provide("inner".to_string());
        assert_eq!(tree.consume::<String>(), Some("inner".to_string()));
        tree.pop_layer();
        assert_eq!(tree.consume::<String>(), Some("outer".to_string()));
    }

    #[test]
    fn test_tree_missing_value() {
        let mut tree = ContextTree::new();
        assert!(tree.consume::<String>().is_none());
        tree.provide(1_u32);
        assert_eq!(tree.consume::<u32>(), Some(1));
        assert!(tree.consume::<u64>().is_none());
    }

    #[test]
    fn test_tree_pop_keeps_base_layer() {
        let mut tree = ContextTree::new();
        tree.push_layer();
        tree.push_layer();
        tree.pop_layer();
        tree.pop_layer();
        // Popping below the base layer is a no-op, never empty.
        tree.pop_layer();
        tree.provide("still works".to_string());
        assert_eq!(tree.consume::<String>(), Some("still works".to_string()));
    }

    #[test]
    fn test_tree_subscribe_notified_on_provide() {
        let mut tree = ContextTree::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let clone = Arc::clone(&calls);
        tree.subscribe::<u8>(move || {
            clone.fetch_add(1, Ordering::SeqCst);
        });
        tree.provide(1_u8);
        tree.provide(2_u8);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_tree_subscribe_typed_isolation() {
        let mut tree = ContextTree::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let clone = Arc::clone(&calls);
        tree.subscribe::<u8>(move || {
            clone.fetch_add(1, Ordering::SeqCst);
        });
        tree.provide("unrelated".to_string());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        tree.provide(1_u8);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_tree_default_impl() {
        let mut tree = ContextTree::default();
        tree.provide(1_u8);
        assert_eq!(tree.consume::<u8>(), Some(1));
    }
}

#[cfg(test)]
mod window_context_tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[gpui::test]
    fn test_window_context_free_functions(cx: &mut gpui::TestAppContext) {
        let window = cx.add_window(|_window, _cx| TestView);
        let _ = window.update(cx, |_, window, _cx| {
            let window_id = window.window_handle().window_id();
            init_window(window_id);
            provide(window_id, "ctx-value".to_string());
            assert_eq!(consume::<String>(window_id), Some("ctx-value".to_string()));
            assert!(consume::<u8>(window_id).is_none());
            push_layer(window_id);
            provide(window_id, 42_u8);
            assert_eq!(consume::<u8>(window_id), Some(42));
            assert_eq!(consume::<String>(window_id), Some("ctx-value".to_string()));
            pop_layer(window_id);
            assert!(consume::<u8>(window_id).is_none());
            let hits = Arc::new(AtomicUsize::new(0));
            let hits_clone = Arc::clone(&hits);
            subscribe::<u64>(window_id, move || {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            });
            provide(window_id, 1_u64);
            assert_eq!(hits.load(Ordering::SeqCst), 1);
            destroy_window(window_id);
            assert!(consume::<String>(window_id).is_none());
        });
    }

    struct TestView;

    impl gpui::Render for TestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }
}
