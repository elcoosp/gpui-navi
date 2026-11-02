//! Suspense boundary: fallback while any loader is pending.

use crate::RouterHandle;
use quoin_ui::Node;

pub struct SuspenseBoundary {
    fallback: Box<dyn Fn() -> Node>,
}

impl SuspenseBoundary {
    #[must_use]
    pub fn new(fallback: impl Fn() -> Node + 'static) -> Self {
        Self {
            fallback: Box::new(fallback),
        }
    }

    /// Builds the boundary's node tree.
    #[must_use]
    pub fn build(self) -> Node {
        let has_pending =
            RouterHandle::try_current().is_some_and(|handle| handle.read().has_pending_loader());
        if has_pending {
            (self.fallback)()
        } else {
            super::Outlet::new().build()
        }
    }
}
