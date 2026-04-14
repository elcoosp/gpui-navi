use navi_router::RouterEvent;
use chrono::{DateTime, Local};

/// A logged navigation event with timestamp.
#[derive(Clone, Debug)]
pub struct LoggedEvent {
    pub timestamp: DateTime<Local>,
    pub event: RouterEvent,
}

impl LoggedEvent {
    pub fn new(event: RouterEvent) -> Self {
        Self {
            timestamp: Local::now(),
            event,
        }
    }
}
