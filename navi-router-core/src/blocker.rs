use crate::location::Location;
use futures::future::BoxFuture;
use std::sync::Arc;
pub type BlockerId = usize;

#[derive(Clone)]
pub struct Blocker {
    pub should_block_fn: Arc<dyn Fn(&Location, &Location) -> BoxFuture<'static, bool> + Send + Sync>,
    pub enable_before_unload: bool,
}

impl Blocker {
    pub fn new<F, Fut>(f: F) -> Self where F: Fn(&Location, &Location) -> Fut + Send + Sync + 'static, Fut: std::future::Future<Output=bool> + Send + 'static {
        Self { should_block_fn: Arc::new(move |a,b| Box::pin(f(a,b))), enable_before_unload: false }
    }
    pub fn new_sync<F>(f: F) -> Self where F: Fn(&Location, &Location) -> bool + Send + Sync + 'static {
        Self::new(move |a,b| { let r = f(a,b); async move { r } })
    }
    pub fn should_allow(&self, a: &Location, b: &Location) -> BoxFuture<'static, bool> {
        let fut = (self.should_block_fn)(a,b);
        Box::pin(async move { !fut.await })
    }
    pub fn with_before_unload(mut self) -> Self { self.enable_before_unload = true; self }
}
