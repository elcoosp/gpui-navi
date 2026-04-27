use crate::location::Location;
use history_navigation::History as BrowserHistory;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct History {
    inner: Arc<Mutex<BrowserHistory<Location>>>,
    listeners: Vec<LocationListener>,
}

impl History {
    pub fn new(initial: Location) -> Self {
        let inner = Arc::new(Mutex::new(BrowserHistory::new(initial)));
        Self {
            inner,
            listeners: Vec::new(),
        }
    }

    pub fn push(&mut self, loc: Location) {
        self.inner.lock().push(loc.clone());
        self.notify_listeners(&loc);
    }
    pub fn replace(&mut self, loc: Location) {
        self.inner.lock().replace(loc.clone());
        self.notify_listeners(&loc);
    }
    pub fn back(&mut self) -> bool {
        if self.inner.lock().back() {
            self.notify_current();
            true
        } else {
            false
        }
    }
    pub fn forward(&mut self) -> bool {
        if self.inner.lock().forward() {
            self.notify_current();
            true
        } else {
            false
        }
    }
    pub fn go(&mut self, delta: isize) {
        self.inner.lock().go(delta);
        self.notify_current();
    }
    pub fn current(&self) -> Location {
        self.inner.lock().current().clone()
    }
    pub fn listen<F: Fn(&Location) + Send + Sync + 'static>(&mut self, f: F) {
        self.listeners.push(Box::new(f));
    }
    pub fn can_go_back(&self) -> bool {
        self.inner.lock().can_go_back()
    }
    pub fn can_go_forward(&self) -> bool {
        self.inner.lock().can_go_forward()
    }

    fn notify_listeners(&self, loc: &Location) {
        for l in &self.listeners {
            l(loc);
        }
    }
    fn notify_current(&self) {
        let loc = self.current();
        self.notify_listeners(&loc);
    }
}

type LocationListener = Box<dyn Fn(&Location) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::Location;

    #[test]
    fn test_push_and_current() {
        let mut history = History::new(Location::new("/"));
        history.push(Location::new("/about"));
        assert_eq!(history.current().pathname, "/about");
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
    }

    #[test]
    fn test_go() {
        let mut history = History::new(Location::new("/"));
        history.push(Location::new("/a"));
        history.push(Location::new("/b"));
        history.go(-2);
        assert_eq!(history.current().pathname, "/");
    }
}
