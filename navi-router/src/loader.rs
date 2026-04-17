// navi-router/src/loader.rs
use gpui::{App, BackgroundExecutor, Task};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub type LoaderError = Box<dyn std::error::Error + Send + Sync>;
pub type LoaderTask = Task<Result<Arc<dyn std::any::Any + Send + Sync>, LoaderError>>;
pub type LoaderFn =
    Box<dyn Fn(&HashMap<String, String>, BackgroundExecutor, &mut App) -> LoaderTask + Send + Sync>;

pub struct CacheEntry {
    pub data: Arc<dyn std::any::Any + Send + Sync>,
    pub inserted_at: Instant,
}

impl CacheEntry {
    pub fn is_stale(&self, stale_time: Option<Duration>) -> bool {
        if let Some(stale) = stale_time {
            self.inserted_at.elapsed() > stale
        } else {
            false
        }
    }
}

#[derive(Default)]
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

#[derive(Clone, Debug)]
pub enum LoaderState {
    Idle,
    Loading { route_id: String },
    Error { route_id: String, message: String },
}
