//! UI-thread task driver shared by every Navi demo host.
//!
//! The router's `RouterHost.spawn` seam hands us `!Send` futures that must
//! run on the thread owning the router state (the UI thread — for Dioxus,
//! the VirtualDom thread). Unlike GPUI's `AsyncApp`, most quoin adapters
//! have no built-in spawn for local futures, so the driver parks them and
//! polls once per shell render:
//!
//! - demo loaders only await quoin's portable timers, which make progress
//!   whenever polled after their deadline (`futures_timer::Delay` on
//!   native, a browser timeout on wasm), and the router's spawned futures
//!   call `handle.refresh()` on completion — so render-driven polling is
//!   sufficient, and no future ever needs a cross-thread waker;
//! - `poll_pending` is called from [`crate::shell`] at the top of every
//!   render, on the thread that installed the router.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Waker};

/// A boxed local future — the router's spawn currency.
pub type LocalFuture = Pin<Box<dyn Future<Output = ()>>>;

/// Parks `!Send` router futures and polls them once per render.
#[derive(Default)]
pub struct LocalDriver {
    pending: RefCell<Vec<LocalFuture>>,
}

impl LocalDriver {
    /// Creates a driver handle (cheap to clone through `Rc`).
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Parks a future for render-driven polling.
    pub fn spawn(self: &Rc<Self>, future: LocalFuture) {
        self.pending.borrow_mut().push(future);
    }

    /// Polls every parked future once (noop waker — timer-driven futures
    /// make progress by being polled after their deadline).
    ///
    /// Returns the number of futures still parked after this pass.
    pub fn poll_pending(self: &Rc<Self>) -> usize {
        let mut pending = self.pending.borrow_mut();
        if pending.is_empty() {
            return 0;
        }
        let waker = Waker::noop();
        let mut cx = Context::from_waker(&waker);
        let mut still_pending = Vec::new();
        for mut future in pending.drain(..) {
            if future.as_mut().poll(&mut cx).is_pending() {
                still_pending.push(future);
            }
        }
        let remaining = still_pending.len();
        *pending = still_pending;
        remaining
    }
}

// Per-window driver slot. The demos are single-window: the shell installs
// the router and driver once, and every render polls the same slot.
thread_local! {
    static DRIVER: RefCell<Option<Rc<LocalDriver>>> = const { RefCell::new(None) };
}

/// Registers the driver for the calling thread (idempotent per install).
pub fn set_thread_driver(driver: Rc<LocalDriver>) {
    DRIVER.with(|slot| *slot.borrow_mut() = Some(driver));
}

/// Polls the thread driver's parked futures, if any.
pub fn poll_router_tasks() {
    DRIVER.with(|slot| {
        if let Some(driver) = slot.borrow().as_ref() {
            driver.poll_pending();
        }
    });
}
