use crate::RouterEvent;
use gpui::{App, Global};
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone)]
pub struct RouterEventLog {
    events: Arc<Mutex<Vec<RouterEvent>>>,
}

impl Global for RouterEventLog {}

pub fn init_event_log(cx: &mut App) {
    cx.set_global(RouterEventLog {
        events: Arc::new(Mutex::new(Vec::new())),
    });
    println!("[navi-router] Event log initialized");
}

pub fn push_event(event: RouterEvent, cx: &mut App) {
    println!("[navi-router] Pushing event: {:?}", event);
    if let Some(log) = cx.try_global::<RouterEventLog>() {
        log.events.lock().push(event);
        // Note: We'll rely on RouterState's own notification to trigger observers.
    }
}

pub fn get_event_log(cx: &App) -> Vec<RouterEvent> {
    cx.try_global::<RouterEventLog>()
        .map(|log| log.events.lock().clone())
        .unwrap_or_default()
}
