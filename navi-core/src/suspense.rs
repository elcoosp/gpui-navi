//! Suspense primitives for handling async data loading states.

/// Represents a pending async operation that may resolve to a value or error.
#[derive(Default)]
pub enum SuspenseState<T> {
    /// No data has been requested yet.
    #[default]
    Idle,
    /// Data is currently being loaded.
    Pending,
    /// Data has been successfully loaded.
    Ready(T),
    /// An error occurred while loading data.
    Error(String),
}

impl<T: Clone> Clone for SuspenseState<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Idle => Self::Idle,
            Self::Pending => Self::Pending,
            Self::Ready(v) => Self::Ready(v.clone()),
            Self::Error(e) => Self::Error(e.clone()),
        }
    }
}

/// Configuration for suspense boundary behavior.
pub struct SuspenseConfig {
    /// Time in milliseconds before showing the pending component.
    pub pending_ms: u64,
    /// Minimum time in milliseconds to show the pending component.
    pub pending_min_ms: u64,
}

impl Default for SuspenseConfig {
    fn default() -> Self {
        Self {
            pending_ms: 1000,
            pending_min_ms: 500,
        }
    }
}
