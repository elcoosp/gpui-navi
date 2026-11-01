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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suspense_state_default_is_idle() {
        let state: SuspenseState<u8> = SuspenseState::default();
        assert!(matches!(state, SuspenseState::Idle));
    }

    #[test]
    fn test_suspense_state_transitions() {
        let mut state = SuspenseState::<String>::Idle;
        assert!(matches!(state, SuspenseState::Idle));
        state = SuspenseState::Pending;
        assert!(matches!(state, SuspenseState::Pending));
        state = SuspenseState::Ready("data".to_string());
        assert!(matches!(state, SuspenseState::Ready(ref v) if v == "data"));
        state = SuspenseState::Error("boom".to_string());
        assert!(matches!(state, SuspenseState::Error(ref e) if e == "boom"));
    }

    #[test]
    fn test_suspense_state_clone() {
        let ready = SuspenseState::Ready(vec![1_u8, 2]);
        let cloned = ready.clone();
        assert!(matches!(cloned, SuspenseState::Ready(v) if v == vec![1_u8, 2]));
        let error = SuspenseState::<u8>::Error("e".into());
        assert!(matches!(error.clone(), SuspenseState::Error(ref e) if e == "e"));
    }

    #[test]
    fn test_suspense_config_defaults() {
        let config = SuspenseConfig::default();
        assert_eq!(config.pending_ms, 1000);
        assert_eq!(config.pending_min_ms, 500);
    }

    #[test]
    fn test_suspense_config_custom() {
        let config = SuspenseConfig {
            pending_ms: 10,
            pending_min_ms: 5,
        };
        assert_eq!(config.pending_ms, 10);
        assert_eq!(config.pending_min_ms, 5);
    }
}
