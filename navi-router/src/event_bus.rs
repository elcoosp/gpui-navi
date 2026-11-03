//! Router event log shared with the devtools timeline.
//!
//! The log is a bounded ring buffer (`Arc<Mutex<VecDeque>>`) owned by the
//! router state; the thread-current handle (installed by
//! [`RouterProvider::install`](crate::RouterProvider::install)) is what the
//! free functions below address, mirroring the classic global API shape.

use crate::RouterEvent;
use chrono::{DateTime, Local};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Default upper bound on remembered events. Long sessions no longer grow
/// the log without bound; the devtools timeline sees the most recent
/// [`DEFAULT_EVENT_LOG_CAPACITY`] events.
pub const DEFAULT_EVENT_LOG_CAPACITY: usize = 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct TimedEvent {
    pub timestamp: DateTime<Local>,
    pub event: RouterEvent,
}

/// A shared, append-only, capacity-bounded event log.
#[derive(Clone)]
pub struct EventLog {
    events: Arc<Mutex<VecDeque<TimedEvent>>>,
    capacity: usize,
}

impl EventLog {
    /// Creates an empty log with the default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_EVENT_LOG_CAPACITY)
    }

    /// Creates an empty log that remembers at most `capacity` events
    /// (oldest evicted first).
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.min(64)))),
            capacity,
        }
    }

    /// Appends an event, evicting the oldest one when the log is full.
    pub fn push(&self, event: RouterEvent) {
        let mut events = self.events.lock();
        if self.capacity == 0 {
            return;
        }
        if events.len() >= self.capacity {
            events.pop_front();
        }
        events.push_back(TimedEvent {
            timestamp: Local::now(),
            event,
        });
    }

    /// A snapshot of the most recent recorded events (bounded by capacity).
    #[must_use]
    pub fn get(&self) -> Vec<TimedEvent> {
        self.events.lock().iter().cloned().collect()
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

    /// The configured capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;

    fn nav_event(path: &str) -> RouterEvent {
        RouterEvent::BeforeNavigate {
            from: None,
            to: Location::new(path),
        }
    }

    #[test]
    fn test_push_and_snapshot() {
        let log = EventLog::new();
        log.push(nav_event("/a"));
        log.push(nav_event("/b"));
        assert_eq!(log.len(), 2);
        let snapshot = log.get();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(
            snapshot[0].event.to().map(|l| l.pathname.clone()),
            Some("/a".into())
        );
        assert_eq!(
            snapshot[1].event.to().map(|l| l.pathname.clone()),
            Some("/b".into())
        );
    }

    #[test]
    fn test_ring_buffer_evicts_oldest() {
        let log = EventLog::with_capacity(3);
        for p in ["/a", "/b", "/c", "/d", "/e"] {
            log.push(nav_event(p));
        }
        assert_eq!(log.len(), 3);
        let snapshot = log.get();
        let paths: Vec<String> = snapshot
            .iter()
            .filter_map(|e| e.event.to().map(|l| l.pathname.clone()))
            .collect();
        assert_eq!(paths, vec!["/c", "/d", "/e"]);
    }

    #[test]
    fn test_zero_capacity_log_is_always_empty() {
        let log = EventLog::with_capacity(0);
        log.push(nav_event("/a"));
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
        assert!(log.get().is_empty());
        assert_eq!(log.capacity(), 0);
    }

    #[test]
    fn test_clear_and_empty() {
        let log = EventLog::new();
        assert!(log.is_empty());
        log.push(nav_event("/a"));
        log.clear();
        assert!(log.is_empty());
        assert_eq!(log.capacity(), DEFAULT_EVENT_LOG_CAPACITY);
    }

    #[test]
    fn test_timed_event_serializes() {
        let log = EventLog::new();
        log.push(nav_event("/a"));
        let json = serde_json::to_string(&log.get()[0]).expect("serialize");
        assert!(json.contains("BeforeNavigate"));
    }

    #[test]
    fn test_push_event_to_log() {
        let log = EventLog::new();
        crate::event_bus::push_event_to(&log, nav_event("/x"));
        assert_eq!(log.len(), 1);
    }
}
