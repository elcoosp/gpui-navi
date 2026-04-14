use crate::location::Location;

/// Unique identifier for a navigation blocker.
pub type BlockerId = usize;

/// A navigation blocker that can prevent route changes.
pub struct Blocker {
    /// Function that returns true if navigation should be blocked.
    pub should_block_fn: Box<dyn Fn(&Location, &Location) -> bool + Send + Sync>,
    /// Whether to enable before-unload handling.
    pub enable_before_unload: bool,
}

impl Blocker {
    pub fn new(
        should_block_fn: impl Fn(&Location, &Location) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            should_block_fn: Box::new(should_block_fn),
            enable_before_unload: false,
        }
    }

    /// Returns true if navigation should be allowed (i.e., not blocked).
    pub fn should_allow(&self, from: &Location, to: &Location) -> bool {
        !(self.should_block_fn)(from, to)
    }

    pub fn with_before_unload(mut self) -> Self {
        self.enable_before_unload = true;
        self
    }
}
