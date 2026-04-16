use crate::RouterEvent;
use chrono::{DateTime, Local};
use gpui::{App, Global};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub struct TimedEvent {
    pub timestamp: DateTime<Local>,
    pub event: RouterEvent,
}

pub struct RouterEventLog {
    events: Arc<Mutex<Vec<TimedEvent>>>,
}

impl Global for RouterEventLog {}

pub fn init_event_log(cx: &mut App) {
    cx.set_global(RouterEventLog {
        events: Arc::new(Mutex::new(Vec::new())),
    });
}

pub fn push_event(event: RouterEvent, cx: &mut App) {
    if let Some(log) = cx.try_global::<RouterEventLog>() {
        log.events.lock().push(TimedEvent {
            timestamp: Local::now(),
            event,
        });
    }
}

pub fn get_event_log(cx: &App) -> Vec<TimedEvent> {
    cx.try_global::<RouterEventLog>()
        .map(|log| log.events.lock().clone())
        .unwrap_or_default()
}

pub fn clear_event_log(cx: &mut App) {
    if let Some(log) = cx.try_global::<RouterEventLog>() {
        log.events.lock().clear();
    }
}
