//! Router event log shared with the devtools timeline.
//!
//! The log is an `Arc<Mutex<Vec>>` handle owned by the router state; the
//! thread-current handle (installed by
//! [`RouterProvider::install`](crate::RouterProvider::install)) is what the
//! free functions below address, mirroring the classic global API shape.

use crate::RouterEvent;
use chrono::{DateTime, Local};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub struct TimedEvent {
    pub timestamp: DateTime<Local>,
    pub event: RouterEvent,
}

/// A shared, append-only event log.
#[derive(Clone, Default)]
pub struct EventLog {
    events: Arc<Mutex<Vec<TimedEvent>>>,
}

impl EventLog {
    /// Creates an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an event.
    pub fn push(&self, event: RouterEvent) {
        self.events.lock().push(TimedEvent {
            timestamp: Local::now(),
            event,
        });
    }

    /// A snapshot of all recorded events.
    #[must_use]
    pub fn get(&self) -> Vec<TimedEvent> {
        self.events.lock().clone()
    }

    /// Number of recorded events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.lock().len()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.lock().is_empty()
    }

    /// Removes all recorded events.
    pub fn clear(&self) {
        self.events.lock().clear();
    }
}

/// Initializes the thread-current event log (no-op: the log lives on the
/// router handle; kept for API parity with the classic `init_event_log`).
pub fn init_event_log() {
    // The event log is created with the router handle; this function is a
    // compatibility no-op.
}

/// Appends an event to the thread-current router's log.
///
/// # Panics
/// Panics when no router is installed on this thread.
pub fn push_event(event: RouterEvent) {
    crate::RouterHandle::current().events().push(event);
}

/// Appends an event to the given log.
pub fn push_event_to(log: &EventLog, event: RouterEvent) {
    log.push(event);
}

/// A snapshot of the thread-current router's event log (empty when no
/// router is installed).
#[must_use]
pub fn get_event_log() -> Vec<TimedEvent> {
    crate::RouterHandle::try_current()
        .map(|handle| handle.events().get())
        .unwrap_or_default()
}

/// Clears the thread-current router's event log.
pub fn clear_event_log() {
    if let Some(handle) = crate::RouterHandle::try_current() {
        handle.events().clear();
    }
}
