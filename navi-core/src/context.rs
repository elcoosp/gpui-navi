use gpui::WindowId;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;

/// A single layer in the context tree, holding type-keyed values.
pub struct ContextLayer {
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl ContextLayer {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn insert<T: 'static>(&mut self, val: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(val));
    }

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
        c.borrow_mut()
            .insert(window_id, ContextTree::new());
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
