// navi-router/src/deep_link.rs
//! Deep link integration with Nexum (feature = "nexum").

use chrono::{DateTime, Local};
use gpui::{AnyWindowHandle, App, AppContext, AsyncApp};
use log::{info, warn};
use nexum_core::Config;
use nexum_gpui::{attach_deep_link, setup_deep_links};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::sync::{broadcast, mpsc};

#[derive(Clone, Debug)]
pub enum DeepLinkStatus {
    Success,
    Blocked,
    ParseError,
}

#[derive(Clone, Debug)]
pub struct DeepLinkEvent {
    pub timestamp: DateTime<Local>,
    pub url: String,
    pub status: DeepLinkStatus,
    pub matched_route: Option<String>,
}

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

// Opaque wrapper so we don't have to expose nexum_core types in your public API
pub struct DeepLinkHandle(pub(crate) nexum_core::DeepLinkHandle);

/// Step 1: Setup OS-level scheme listener.
/// MUST be called BEFORE `app.run()` consumes the Application handle!
pub fn setup(app: &gpui::Application, schemes: Vec<String>) -> DeepLinkHandle {
    let config = Config {
        schemes,
        app_links: vec![],
    };
    info!(
        "🚀 Initializing Nexum deep links for schemes: {:?}",
        config.schemes
    );
    let handle = setup_deep_links(app, config);
    DeepLinkHandle(handle)
}

/// Step 2: Attach the GPUI entity listener.
/// MUST be called INSIDE `app.run()` / `cx.open_window()` where cx and window are available.
pub fn attach(handle: DeepLinkHandle, window: AnyWindowHandle, cx: &mut App) {
    // Initialize broadcast channel for DevTools
    let (tx, _rx) = broadcast::channel(16);
    *EVENT_TX.lock().unwrap() = Some(tx);

    // Create channel to bridge the 2-arg callback and the async task
    let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<String>(16);

    // Dummy entity required by attach_deep_link
    let receiver = cx.new(|_| ());

    attach_deep_link(handle.0, receiver, cx, move |_receiver, url| {
        info!(
            "🎉 [Nexum Callback] OS triggered deep link! URL received: {}",
            url
        );
        if mpsc_tx.try_send(url).is_err() {
            warn!("⚠️ [Nexum Callback] Failed to send URL to async channel");
        }
    });

    // Spawn async task to navigate
    cx.spawn(move |cx: &mut AsyncApp| {
        let cx = cx.clone();
        async move {
            while let Some(url) = mpsc_rx.recv().await {
                info!("🌐 [Async Task] Picked up URL: {}", url);

                let (status, matched_route) = match crate::Location::from_url(&url) {
                    Ok(loc) => {
                        info!("✅ [Async Task] Parsed Location: {:?}", loc.pathname);

                        let nav_result = cx.update(|cx| {
                            let navigator = crate::Navigator::new(window);
                            navigator.push_location(loc, cx);
                            crate::RouterState::global(cx)
                                .current_match
                                .as_ref()
                                .map(|(_, node)| node.id.clone())
                        });

                        match nav_result {
                            Some(route_id) => {
                                info!(
                                    "🟢 [Async Task] Navigation succeeded! Route: {:?}",
                                    route_id
                                );
                                (DeepLinkStatus::Success, Some(route_id))
                            }
                            None => {
                                warn!("❌ [Async Task] cx.update returned None!");
                                (DeepLinkStatus::Success, None)
                            }
                        }
                    }
                    Err(e) => {
                        warn!("❌ [Async Task] Failed to parse URL '{}': {}", url, e);
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
