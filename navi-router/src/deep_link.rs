// navi-router/src/deep_link.rs
//! Deep link integration with Nexum (feature = "nexum").

use chrono::{DateTime, Local};
use gpui::{AnyWindowHandle, App, AsyncApp};
use log::{error, warn};
pub use nexum_core::Config;
use nexum_gpui::setup_deep_links;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::sync::broadcast;

/// Status of a deep link processing attempt.
#[derive(Clone, Debug)]
pub enum DeepLinkStatus {
    Success,
    Blocked,
    ParseError,
}

/// Event emitted when a deep link is processed.
#[derive(Clone, Debug)]
pub struct DeepLinkEvent {
    pub timestamp: DateTime<Local>,
    pub url: String,
    pub status: DeepLinkStatus,
    pub matched_route: Option<String>,
}

/// Global broadcast sender for deep link events.
static EVENT_TX: Lazy<Mutex<Option<broadcast::Sender<DeepLinkEvent>>>> =
    Lazy::new(|| Mutex::new(None));

fn emit_event(event: DeepLinkEvent) {
    if let Some(tx) = EVENT_TX.lock().unwrap().as_ref() {
        let _ = tx.send(event);
    }
}

/// Subscribe to deep link events (for DevTools).
pub fn subscribe_events() -> Option<broadcast::Receiver<DeepLinkEvent>> {
    EVENT_TX.lock().unwrap().as_ref().map(|tx| tx.subscribe())
}

/// Initialize deep link handling.
pub fn init(app: &gpui::Application, schemes: Vec<String>, window: AnyWindowHandle, cx: &mut App) {
    let (tx, _rx) = broadcast::channel(16);
    *EVENT_TX.lock().unwrap() = Some(tx);

    // Construct the config internally
    let config = Config {
        schemes,
        app_links: vec![],
    };

    let handle = setup_deep_links(app, config);

    cx.spawn(move |cx: &mut AsyncApp| {
        let cx = cx.clone();
        async move {
            while let Some(url) = handle.recv().await {
                let loc_result = crate::Location::from_url(&url);

                let (status, matched_route) = match loc_result {
                    Ok(loc) => {
                        let nav_result = cx.update(|cx| {
                            let navigator = crate::Navigator::new(window);
                            navigator.push_location(loc, cx);
                            crate::RouterState::global(cx)
                                .current_match
                                .as_ref()
                                .map(|(_, node)| node.id.clone())
                        });

                        match nav_result {
                            Some(route_id) => (DeepLinkStatus::Success, Some(route_id)),
                            None => {
                                error!("Navigation update failed: app context unavailable");
                                (DeepLinkStatus::Success, None)
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse deep link URL '{}': {}", url, e);
                        (DeepLinkStatus::ParseError, None)
                    }
                };

                emit_event(DeepLinkEvent {
                    timestamp: Local::now(),
                    url,
                    status,
                    matched_route,
                });
            }
        }
    })
    .detach();
}
