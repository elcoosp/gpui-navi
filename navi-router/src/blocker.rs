use crate::location::Location;
use futures::future::BoxFuture;
use std::sync::Arc;

/// Unique identifier for a navigation blocker.
pub type BlockerId = usize;

/// A navigation blocker that can prevent route changes, optionally asynchronously.
#[derive(Clone)]
pub struct Blocker {
    /// Function that returns a future resolving to true if navigation should be blocked.
    pub should_block_fn: Arc<dyn Fn(&Location, &Location) -> BoxFuture<'static, bool> + Send + Sync>,
    /// Whether to enable before-unload handling.
    pub enable_before_unload: bool,
}

impl Blocker {
    /// Create a new async blocker.
    pub fn new<F, Fut>(should_block_fn: F) -> Self
    where
        F: Fn(&Location, &Location) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = bool> + Send + 'static,
    {
        Self {
            should_block_fn: Arc::new(move |from, to| Box::pin(should_block_fn(from, to))),
            enable_before_unload: false,
        }
    }

    /// Create a synchronous blocker (convenience wrapper).
    pub fn new_sync<F>(should_block_fn: F) -> Self
    where
        F: Fn(&Location, &Location) -> bool + Send + Sync + 'static,
    {
        Self::new(move |from, to| {
            let result = should_block_fn(from, to);
            async move { result }
        })
    }

    /// Returns a future that resolves to true if navigation should be allowed (i.e., not blocked).
    pub fn should_allow(&self, from: &Location, to: &Location) -> BoxFuture<'static, bool> {
        let fut = (self.should_block_fn)(from, to);
        Box::pin(async move { !fut.await })
    }

    pub fn with_before_unload(mut self) -> Self {
        self.enable_before_unload = true;
        self
    }
}
