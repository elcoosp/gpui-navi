use crate::location::Location;
use gpui::WindowId;
use history_navigation::History as BrowserHistory;
use parking_lot::Mutex;
use std::sync::Arc;

/// History manager wrapping history-navigation crate with listener support.
pub struct History {
    inner: Arc<Mutex<BrowserHistory<Location>>>,
    window_id: WindowId,
    listeners: Vec<LocationListener>,
}

impl History {
    pub fn new(initial: Location, window_id: WindowId) -> Self {
        let inner = Arc::new(Mutex::new(BrowserHistory::new(initial)));
        Self {
            inner,
            window_id,
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

    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    fn notify_listeners(&self, loc: &Location) {
        for listener in &self.listeners {
            listener(loc);
        }
    }

    fn notify_current(&self) {
        let loc = self.current();
        self.notify_listeners(&loc);
    }
}

type LocationListener = Box<dyn Fn(&crate::Location) + Send + Sync>;
