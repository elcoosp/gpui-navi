I'm using the writing-plans skill to create the final implementation plan.

---

# Navi‑Nexum Deep Link Integration – Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional deep‑link support to Navi router via Nexum, with a dedicated DevTools deep‑link viewer and showcase in the example app.

**Architecture:** A feature‑gated `deep_link` module in `navi-router` initializes Nexum, spawns a background task that listens for URLs, and navigates via `Navigator`. Events are emitted over a broadcast channel. The `navi-devtools` crate (with `nexum` feature) subscribes to this channel and provides a `DeepLinkView` component. The example app demonstrates the integration.

**Tech Stack:** Rust, GPUI, Nexum (core + gpui adapter), Navi router & devtools, tokio (broadcast), chrono, log.

---

## Chunk 1: Core Integration in `navi-router`

### Task 1: Add `nexum` feature flag and dependencies

**Files:**
- Modify: `navi-router/Cargo.toml`

- [ ] **Step 1: Add `log` to regular dependencies**

```toml
[dependencies]
log = "0.4"
# ... other deps
```

- [ ] **Step 2: Add `nexum` feature with optional dependencies**

```toml
[features]
default = []
validator = ["dep:validator"]
garde = ["dep:garde"]
validify = ["dep:validify"]
valico = ["dep:valico", "dep:schemars"]
nexum = ["dep:nexum-core", "dep:nexum-gpui", "dep:tokio", "dep:chrono", "dep:once_cell"]
```

- [ ] **Step 3: Add optional dependencies**

```toml
nexum-core = { path = "../../nexum-core", optional = true }
nexum-gpui = { path = "../../nexum-gpui", optional = true }
tokio = { version = "1", features = ["sync", "rt"], optional = true }
chrono = { version = "0.4", optional = true }
once_cell = { version = "1.20", optional = true }
```

- [ ] **Step 4: Verify build without feature**

Run: `cargo check -p navi-router`
Expected: Compiles successfully, no new warnings.

- [ ] **Step 5: Verify build with feature**

Run: `cargo check -p navi-router --features nexum`
Expected: Compiles successfully, Nexum crates pulled in.

- [ ] **Step 6: Commit**

```bash
git add navi-router/Cargo.toml
git commit -m "feat(navi-router): add 'nexum' feature with Nexum deps and log"
```

### Task 2: Create `deep_link` module with event types and channel

**Files:**
- Create: `navi-router/src/deep_link.rs`
- Modify: `navi-router/src/lib.rs`

- [ ] **Step 1: Create `src/deep_link.rs` with feature‑gated content**

```rust
// navi-router/src/deep_link.rs
//! Deep link integration with Nexum (feature = "nexum").

use chrono::{DateTime, Local};
use gpui::{AnyWindowHandle, App, AsyncApp};
use log::{error, warn};
use nexum_core::Config;
use nexum_gpui::setup_deep_links;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::sync::broadcast;

pub use nexum_core::Config;

/// Status of a deep link processing attempt.
#[derive(Clone, Debug)]
pub enum DeepLinkStatus {
    /// Successfully navigated.
    Success,
    /// Navigation was blocked by a router blocker.
    Blocked,
    /// URL parsing failed.
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
///
/// This function must be called **after** `RouterProvider` has been created.
/// It registers URL schemes and spawns a background task that listens for incoming URLs.
pub fn init(app: &gpui::Application, config: Config, window: AnyWindowHandle, cx: &mut App) {
    // Create broadcast channel with capacity 16
    let (tx, _rx) = broadcast::channel(16);
    *EVENT_TX.lock().unwrap() = Some(tx);

    let handle = setup_deep_links(app, config);

    cx.spawn(|mut cx: AsyncApp| async move {
        while let Some(url) = handle.recv().await {
            let loc_result = crate::Location::from_url(&url);

            let (status, matched_route) = match loc_result {
                Ok(loc) => {
                    // Navigate on the main thread
                    let nav_result = cx.update(|cx| {
                        let navigator = crate::Navigator::new(window);
                        navigator.push_location(loc, cx);
                        // After navigation, try to get the matched route ID
                        crate::RouterState::global(cx)
                            .current_match
                            .as_ref()
                            .map(|(_, node)| node.id.clone())
                    });

                    match nav_result {
                        Ok(route_id) => (DeepLinkStatus::Success, route_id),
                        Err(e) => {
                            error!("Failed to navigate after deep link: {}", e);
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
    })
    .detach();
}
```

- [ ] **Step 2: Add module export in `src/lib.rs`**

```rust
#[cfg(feature = "nexum")]
pub mod deep_link;
```

- [ ] **Step 3: Verify compilation with feature**

Run: `cargo check -p navi-router --features nexum`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add navi-router/src/deep_link.rs navi-router/src/lib.rs
git commit -m "feat(navi-router): implement deep_link module with event emission"
```

### Task 3: Update `example-app` to enable and use deep linking

**Files:**
- Modify: `example-app/Cargo.toml`
- Modify: `example-app/src/main.rs`

- [ ] **Step 1: Add `nexum` feature to `example-app/Cargo.toml`**

```toml
[features]
default = []
validator = ["navi-router/validator", "dep:validator"]
garde = ["navi-router/garde", "dep:garde"]
validify = ["navi-router/validify", "dep:validify"]
valico = ["navi-router/valico", "dep:valico", "dep:schemars"]
nexum = ["navi-router/nexum"]
```

- [ ] **Step 2: Capture `Application` handle before `run` in `example-app/src/main.rs`**

```rust
fn main() {
    env_logger::init();
    log::info!("Starting Navi example app with file-based routing");

    let app = gpui_platform::application();
    app.with_assets(Assets)
        .run(|cx: &mut App| {
            // ... existing setup ...
        });
}
```

- [ ] **Step 3: Add deep link initialization inside `cx.open_window` after `RouterProvider` creation**

Inside the `cx.open_window` closure, after `let router_provider = ...` and `route_tree::register_routes(cx);`:

```rust
#[cfg(feature = "nexum")]
{
    use navi_router::deep_link;
    let config = deep_link::Config {
        schemes: vec!["naviapp".to_string()],
        app_links: vec![],
    };
    deep_link::init(&app, config, window_handle, cx);
}
```

- [ ] **Step 4: Add a UI hint in the root layout**

In `example-app/src/routes/__root.rs`, add a conditional element:

```rust
#[cfg(feature = "nexum")]
div()
    .text_sm()
    .text_color(rgb(0xa6adc8))
    .child("Try: open naviapp://settings")
```

- [ ] **Step 5: Verify compilation and run**

Run: `cargo run -p example-app --features nexum`
Expected: App launches. Trigger deep link (macOS: `open naviapp://settings`) and observe navigation.

- [ ] **Step 6: Commit**

```bash
git add example-app/Cargo.toml example-app/src/main.rs example-app/src/routes/__root.rs
git commit -m "feat(example-app): integrate deep link initialization with 'nexum' feature"
```

---

## Chunk 2: DevTools Deep Link Viewer

### Task 4: Add `nexum` feature to `navi-devtools`

**Files:**
- Modify: `navi-devtools/Cargo.toml`

- [ ] **Step 1: Add `nexum` feature with dependencies**

```toml
[features]
nexum = ["navi-router/nexum", "dep:tokio", "dep:chrono"]
```

- [ ] **Step 2: Add optional dependencies**

```toml
tokio = { version = "1", features = ["sync"], optional = true }
chrono = { version = "0.4", optional = true }
```

- [ ] **Step 3: Verify build**

Run: `cargo check -p navi-devtools` and `cargo check -p navi-devtools --features nexum`

- [ ] **Step 4: Commit**

```bash
git add navi-devtools/Cargo.toml
git commit -m "feat(navi-devtools): add 'nexum' feature with tokio/chrono deps"
```

### Task 5: Create `DeepLinkView` component

**Files:**
- Create: `navi-devtools/src/deep_link_view.rs`
- Modify: `navi-devtools/src/lib.rs`

- [ ] **Step 1: Create `src/deep_link_view.rs`**

```rust
// navi-devtools/src/deep_link_view.rs
//! Deep link viewer component (feature = "nexum").

use chrono::{DateTime, Local};
use gpui::*;
use navi_router::deep_link::{DeepLinkEvent, DeepLinkStatus};

pub struct DeepLinkView {
    events: Vec<DeepLinkEvent>,
    receiver: Option<tokio::sync::broadcast::Receiver<DeepLinkEvent>>,
}

impl DeepLinkView {
    pub fn new() -> Self {
        let receiver = navi_router::deep_link::subscribe_events();
        Self {
            events: Vec::new(),
            receiver,
        }
    }

    fn clear(&mut self, cx: &mut ViewContext<Self>) {
        self.events.clear();
        cx.notify();
    }
}

impl Render for DeepLinkView {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // Drain receiver to update events list
        if let Some(rx) = &mut self.receiver {
            while let Ok(event) = rx.try_recv() {
                self.events.push(event);
                if self.events.len() > 100 {
                    self.events.remove(0);
                }
            }
        }

        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().font_weight(FontWeight::BOLD).child("Deep Links"))
                    .child(
                        div()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _event, cx| this.clear(cx)))
                            .child("Clear"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(self.events.iter().rev().map(|event| {
                        let (status_icon, status_color) = match event.status {
                            DeepLinkStatus::Success => ("✅", 0x6a9955),
                            DeepLinkStatus::Blocked => ("⏸️", 0xdcdcaa),
                            DeepLinkStatus::ParseError => ("❌", 0xf44747),
                        };
                        let time = event.timestamp.format("%H:%M:%S").to_string();
                        let route_info = event
                            .matched_route
                            .as_ref()
                            .map(|id| format!(" → {}", id))
                            .unwrap_or_default();

                        div()
                            .flex()
                            .gap_2()
                            .child(div().text_color(rgb(0x808080)).child(time))
                            .child(div().text_color(rgb(status_color)).child(status_icon))
                            .child(div().child(format!("{}{}", event.url, route_info)))
                    })),
            )
    }
}
```

- [ ] **Step 2: Export component in `src/lib.rs`**

```rust
#[cfg(feature = "nexum")]
pub mod deep_link_view;
#[cfg(feature = "nexum")]
pub use deep_link_view::DeepLinkView;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p navi-devtools --features nexum`

- [ ] **Step 4: Commit**

```bash
git add navi-devtools/src/deep_link_view.rs navi-devtools/src/lib.rs
git commit -m "feat(navi-devtools): add DeepLinkView component for nexum feature"
```

### Task 6: Integrate `DeepLinkView` into example app

**Files:**
- Modify: `example-app/src/main.rs`

- [ ] **Step 1: Add `DeepLinkView` to the root view**

In `example-app/src/main.rs`, modify the `AppView` struct and its `Render` impl:

```rust
struct AppView {
    router_provider: RouterProvider,
    devtools: Entity<DevtoolsState>,
    #[cfg(feature = "nexum")]
    deep_link_view: View<navi_devtools::DeepLinkView>,
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .child(self.router_provider.clone().child(Outlet::new()))
            .child(self.devtools.clone())
            #[cfg(feature = "nexum")]
            .child(self.deep_link_view.clone())
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
```

- [ ] **Step 2: Initialize `DeepLinkView` when creating the root view**

In `main.rs`, inside `cx.open_window` after creating `devtools`:

```rust
let query_client = RouterState::global(cx).query_client.clone();
let devtools = cx.new(|cx| DevtoolsState::new(query_client, cx));

#[cfg(feature = "nexum")]
let deep_link_view = cx.new(|_cx| navi_devtools::DeepLinkView::new());

let root_view = cx.new(|_cx| AppView {
    router_provider,
    devtools,
    #[cfg(feature = "nexum")]
    deep_link_view,
});
```

- [ ] **Step 3: Run example app with feature**

Run: `cargo run -p example-app --features nexum`
Expected: Deep link viewer panel appears (stacked vertically with devtools). Trigger a deep link and see events logged.

- [ ] **Step 4: Commit**

```bash
git add example-app/src/main.rs
git commit -m "feat(example-app): integrate DeepLinkView when nexum feature is active"
```

---

## Final Verification

- [ ] Run `cargo build --workspace` (without `nexum`) → no new dependencies pulled, compiles clean.
- [ ] Run `cargo build --workspace --features nexum` → all crates compile.
- [ ] Run `cargo run -p example-app --features nexum` → deep links navigate and viewer displays events.

---

**Plan complete.** Ready for execution.
