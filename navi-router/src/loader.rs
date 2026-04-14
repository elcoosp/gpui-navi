use gpui::{App, Task};
use std::collections::HashMap;
use std::sync::Arc;

pub type LoaderError = Box<dyn std::error::Error + Send + Sync>;

pub enum LoaderResult {
    Pending(Task<Result<Arc<dyn std::any::Any + Send + Sync>, LoaderError>>),
    Ready(Arc<dyn std::any::Any + Send + Sync>),
}

pub type LoaderFn = Box<
    dyn Fn(&mut App) -> Task<Result<Arc<dyn std::any::Any + Send + Sync>, LoaderError>>
        + Send
        + Sync,
>;

pub struct LoaderRegistry {
    loaders: HashMap<String, LoaderFn>,
}

impl LoaderRegistry {
    pub fn new() -> Self {
        Self {
            loaders: HashMap::new(),
        }
    }

    pub fn insert(&mut self, route_id: &str, loader: LoaderFn) {
        self.loaders.insert(route_id.to_string(), loader);
    }

    pub fn get(&self, route_id: &str) -> Option<&LoaderFn> {
        self.loaders.get(route_id)
    }
}
