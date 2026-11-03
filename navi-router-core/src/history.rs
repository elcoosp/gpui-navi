use crate::location::Location;
use history_navigation::History as BrowserHistory;

/// Browser-style history stack.
///
/// The stack is owned exclusively by [`crate::RouterCore`] (every mutation
/// goes through `&mut self`), so the old `Arc<Mutex<...>>` wrapper was pure
/// overhead: locking and refcount traffic on a single-owner structure.
pub struct History {
    inner: BrowserHistory<Location>,
    listeners: Vec<LocationListener>,
}

impl History {
    #[must_use]
    pub fn new(initial: Location) -> Self {
        Self {
            inner: BrowserHistory::new(initial),
            listeners: Vec::new(),
        }
    }

    pub fn push(&mut self, loc: Location) {
        // Notify first so the entry can be moved into the store unchanged.
        self.notify_listeners(&loc);
        self.inner.push(loc);
    }

    pub fn replace(&mut self, loc: Location) {
        self.notify_listeners(&loc);
        self.inner.replace(loc);
    }

    pub fn back(&mut self) -> bool {
        if self.inner.back() {
            self.notify_current();
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) -> bool {
        if self.inner.forward() {
            self.notify_current();
            true
        } else {
            false
        }
    }

    pub fn go(&mut self, delta: isize) {
        self.inner.go(delta);
        self.notify_current();
    }

    #[must_use]
    pub fn current(&self) -> Location {
        self.inner.current().clone()
    }

    /// The current location by reference — avoids the full `Location` clone
    /// in read-only paths (adapters, matching, active-state checks).
    #[must_use]
    pub fn current_ref(&self) -> &Location {
        self.inner.current()
    }

    pub fn listen<F: Fn(&Location) + Send + Sync + 'static>(&mut self, f: F) {
        self.listeners.push(Box::new(f));
    }

    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.inner.can_go_back()
    }

    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.inner.can_go_forward()
    }

    /// Number of entries currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the history has no entries (never the case after `new`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn notify_listeners(&self, loc: &Location) {
        for l in &self.listeners {
            l(loc);
        }
    }

    fn notify_current(&self) {
        // Borrowed read of the current entry: no `Location` clone.
        let loc = self.inner.current();
        self.notify_listeners(loc);
    }
}

type LocationListener = Box<dyn Fn(&Location) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::Location;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_push_and_current() {
        let mut history = History::new(Location::new("/"));
        history.push(Location::new("/about"));
        assert_eq!(history.current().pathname, "/about");
        assert_eq!(history.len(), 2);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_back_and_forward() {
        let mut history = History::new(Location::new("/"));
        history.push(Location::new("/about"));
        assert!(history.back());
        assert_eq!(history.current().pathname, "/");
        assert!(history.forward());
        assert_eq!(history.current().pathname, "/about");
    }

    #[test]
    fn test_back_at_start_is_noop() {
        let mut history = History::new(Location::new("/"));
        assert!(!history.back());
        assert!(!history.forward());
    }

    #[test]
    fn test_can_go_back_and_forward() {
        let mut history = History::new(Location::new("/"));
        assert!(!history.can_go_back());
        history.push(Location::new("/about"));
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());
        history.back();
        assert!(history.can_go_forward());
    }

    #[test]
    fn test_replace() {
        let mut history = History::new(Location::new("/"));
        history.replace(Location::new("/about"));
        assert_eq!(history.current().pathname, "/about");
        assert!(!history.can_go_back());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_go() {
        let mut history = History::new(Location::new("/"));
        history.push(Location::new("/a"));
        history.push(Location::new("/b"));
        history.go(-2);
        assert_eq!(history.current().pathname, "/");
        history.go(10);
        assert_eq!(history.current().pathname, "/b");
        history.go(-10);
        assert_eq!(history.current().pathname, "/");
    }

    #[test]
    fn test_push_truncates_forward_entries() {
        let mut history = History::new(Location::new("/"));
        history.push(Location::new("/a"));
        history.push(Location::new("/b"));
        history.back();
        history.push(Location::new("/c"));
        assert!(!history.can_go_forward());
        assert_eq!(history.current().pathname, "/c");
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_listeners_notified_on_push_replace_and_traversal() {
        let mut history = History::new(Location::new("/"));
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_clone = Arc::clone(&seen);
        history.listen(move |loc: &Location| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            seen_clone.lock().expect("seen").push(loc.pathname.clone());
        });
        history.push(Location::new("/a"));
        history.replace(Location::new("/b"));
        history.back();
        history.forward();
        history.go(-1);
        assert_eq!(calls.load(Ordering::SeqCst), 5);
        assert_eq!(
            *seen.lock().expect("seen"),
            vec!["/a", "/b", "/", "/b", "/"]
        );
    }

    #[test]
    fn test_history_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<History>();
    }

    #[test]
    fn test_current_ref_matches_current() {
        let mut history = History::new(Location::new("/a?b=1"));
        assert_eq!(history.current_ref().pathname, "/a");
        assert_eq!(history.current_ref().search["b"], serde_json::json!(1));
        history.push(Location::new("/c#frag"));
        assert_eq!(history.current_ref().pathname, "/c");
        assert_eq!(history.current_ref().hash, "frag");
        assert_eq!(history.current(), history.current_ref().clone());
    }
}
