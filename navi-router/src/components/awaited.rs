//! Awaited: suspense-style loader-data boundary rendering a `Node`.

use crate::{RouteDef, RouterHandle};
use quoin_ui::Node;
use std::marker::PhantomData;

pub struct Awaited<R: RouteDef> {
    fallback: Option<Box<dyn Fn() -> Node>>,
    child: Option<Box<dyn Fn(R::LoaderData) -> Node>>,
    _phantom: PhantomData<fn() -> R>,
}

impl<R: RouteDef> Awaited<R> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            fallback: None,
            child: None,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    pub fn fallback(mut self, f: impl Fn() -> Node + 'static) -> Self {
        self.fallback = Some(Box::new(f));
        self
    }

    #[must_use]
    pub fn child(mut self, f: impl Fn(R::LoaderData) -> Node + 'static) -> Self {
        self.child = Some(Box::new(f));
        self
    }

    /// Builds the boundary's node tree: the child when loader data is
    /// present, otherwise the fallback (or an empty node).
    #[must_use]
    pub fn build(self) -> Node {
        log::debug!("Awaited::build for {}", std::any::type_name::<R>());
        let data =
            RouterHandle::try_current().and_then(|handle| handle.read().get_loader_data::<R>());
        log::debug!("Awaited data present: {}", data.is_some());

        if let Some(data) = data {
            if let Some(child_fn) = &self.child {
                return child_fn(data.clone());
            }
            return Node::div().build();
        }
        if let Some(fallback_fn) = &self.fallback {
            return fallback_fn();
        }
        Node::div().build()
    }
}

impl<R: RouteDef> Default for Awaited<R> {
    fn default() -> Self {
        Self::new()
    }
}
