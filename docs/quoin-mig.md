# Navi-QUOIN Unified Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `navi-router` into a single `navi-quoin` crate that contains a generic router core and framework‑agnostic UI components written once with `quoin` macros, eliminating per‑framework adapters.

**Architecture:** The new `navi-quoin` crate uses `quoin`'s reactive primitives for state and `quoin-ui` macros for rendering. Components like `Link` and `Outlet` are defined with `component!` and `quoin_render!`, compiling to GPUI, Leptos, or Dioxus based on feature flags. A thin `navi-router` shim preserves backward compatibility.

**Tech Stack:** Rust, `quoin` (reactive core), `quoin-ui` (UCP traits), `quoin-macros` (component transpiler), `rs-query` (data fetching), `gpui` (existing UI target).

---

## Chunk 1: Project Setup and Core Extraction

### Task 1.1: Create `navi-quoin` Crate

**Files:**
- Create: `navi-quoin/Cargo.toml`
- Create: `navi-quoin/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add `navi-quoin` to workspace members**

Add `"navi-quoin"` to the `members` array in the root `Cargo.toml`.

- [ ] **Step 2: Create `navi-quoin/Cargo.toml` with dependencies**

```toml
[package]
name = "navi-quoin"
version = "0.1.0"
edition = "2021"

[dependencies]
quoin = { path = "../quoin" }
quoin-ui = { path = "../quoin-ui" }
quoin-macros = { path = "../quoin-macros" }
rs-query = { git = "https://github.com/elcoosp/rs-query", features = ["quoin"] }

serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
futures = "0.3"
log = "0.4"
thiserror = "1.0"
once_cell = "1.19"
url = "2.5"

[features]
default = []
gpui = ["dep:gpui", "dep:gpui-component", "dep:quoin-gpui", "dep:quoin-ui-gpui"]
leptos = ["dep:leptos", "dep:quoin-leptos"]
dioxus = ["dep:dioxus", "dep:quoin-dioxus"]

[dependencies.gpui]
git = "https://github.com/zed-industries/zed"
optional = true

[dependencies.gpui-component]
git = "https://github.com/longbridge/gpui-component"
optional = true

[dependencies.quoin-gpui]
path = "../quoin-gpui"
optional = true

[dependencies.quoin-ui-gpui]
path = "../quoin-ui-gpui"
optional = true

[dependencies.leptos]
version = "0.8"
optional = true

[dependencies.quoin-leptos]
path = "../quoin-leptos"
optional = true

[dependencies.dioxus]
version = "0.7"
optional = true

[dependencies.quoin-dioxus]
path = "../quoin-dioxus"
optional = true
```

- [ ] **Step 3: Create directory structure**

```bash
mkdir -p navi-quoin/src/core
mkdir -p navi-quoin/src/components
mkdir -p navi-quoin/src/glue
```

- [ ] **Step 4: Create placeholder `lib.rs`**

```rust
// navi-quoin/src/lib.rs
pub mod core;
pub mod components;
#[cfg(feature = "gpui")]
pub mod glue;
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p navi-quoin
```

Expected: Compiles successfully with no errors.

- [ ] **Step 6: Commit**

```bash
git add navi-quoin/ Cargo.toml
git commit -m "chore: scaffold navi-quoin crate"
```

### Task 1.2: Move Pure Core Files from `navi-router`

**Files:**
- Copy: `navi-router/src/blocker.rs` → `navi-quoin/src/core/blocker.rs`
- Copy: `navi-router/src/location.rs` → `navi-quoin/src/core/location.rs`
- Copy: `navi-router/src/radix_tree.rs` → `navi-quoin/src/core/radix_tree.rs`
- Copy: `navi-router/src/route_tree.rs` → `navi-quoin/src/core/route_tree.rs`
- Copy: `navi-router/src/redirect.rs` → `navi-quoin/src/core/redirect.rs`
- Copy: `navi-router/src/validation.rs` → `navi-quoin/src/core/validation.rs`
- Create: `navi-quoin/src/core/history.rs`
- Create: `navi-quoin/src/core/mod.rs`

- [ ] **Step 1: Copy pure files verbatim**

```bash
cp navi-router/src/blocker.rs navi-quoin/src/core/
cp navi-router/src/location.rs navi-quoin/src/core/
cp navi-router/src/radix_tree.rs navi-quoin/src/core/
cp navi-router/src/route_tree.rs navi-quoin/src/core/
cp navi-router/src/redirect.rs navi-quoin/src/core/
cp navi-router/src/validation.rs navi-quoin/src/core/
```

- [ ] **Step 2: Clean `history.rs` and place in `core/`**

Create `navi-quoin/src/core/history.rs` with the GPUI‑free version (remove `window_id` field):

```rust
// navi-quoin/src/core/history.rs
use crate::core::location::Location;
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
        for listener in &self.listeners {
            listener(loc);
        }
    }

    fn notify_current(&self) {
        let loc = self.current();
        self.notify_listeners(&loc);
    }
}

type LocationListener = Box<dyn Fn(&Location) + Send + Sync>;
```

- [ ] **Step 3: Create `core/mod.rs` to re-export all modules**

```rust
// navi-quoin/src/core/mod.rs
pub mod blocker;
pub mod history;
pub mod location;
pub mod radix_tree;
pub mod redirect;
pub mod route_tree;
pub mod validation;
pub mod state;
pub mod navigator;
pub mod event_bus;

pub use blocker::*;
pub use history::*;
pub use location::*;
pub use radix_tree::*;
pub use redirect::*;
pub use route_tree::*;
pub use validation::*;
pub use state::*;
pub use navigator::*;
pub use event_bus::*;
```

- [ ] **Step 4: Update imports in copied files**

In each moved file, replace `crate::` imports with `crate::core::`. For example, in `route_tree.rs`:

```rust
use crate::core::location::Location;
use crate::core::redirect::{NotFound, Redirect};
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p navi-quoin
```

Expected: Compiles successfully.

- [ ] **Step 6: Commit**

```bash
git add navi-quoin/src/core/
git commit -m "feat(navi-quoin): move pure core files from navi-router"
```

---

## Chunk 2: Generic Router Core

### Task 2.1: Implement `Router<C: ReactiveContext>`

**Files:**
- Create: `navi-quoin/src/core/state.rs`

- [ ] **Step 1: Define the generic `Router` struct**

```rust
// navi-quoin/src/core/state.rs
use quoin::{ReactiveContext, Signal, Executor};
use crate::core::{
    History, Location, RouteTree, RouteNode, Blocker, BlockerId,
    NavigateOptions, RouterEvent, NotFoundMode,
};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

pub struct Router<C: ReactiveContext> {
    ctx: C,
    history: History,
    route_tree: Rc<RouteTree>,
    current_match: C::Signal<Option<(HashMap<String, String>, RouteNode)>>,
    pending_navigation: C::Signal<Option<Location>>,
    blockers: HashMap<BlockerId, Blocker>,
    events: Vec<Box<dyn Fn(RouterEvent) + Send + Sync>>,
    next_blocker_id: BlockerId,
    not_found_mode: NotFoundMode,
    not_found_data: C::Signal<Option<serde_json::Value>>,
    default_pending_ms: u64,
    default_pending_min_ms: u64,
    // loader integration will be added later
}

impl<C: ReactiveContext> Router<C> {
    pub fn new(
        ctx: C,
        initial: Location,
        route_tree: Rc<RouteTree>,
        options: RouterOptions,
    ) -> Self {
        let current_match = ctx.create_signal(None);
        let pending_navigation = ctx.create_signal(None);
        let not_found_data = ctx.create_signal(None);

        // Set initial match
        let initial_match = route_tree
            .match_path(&initial.pathname)
            .map(|(params, node)| (params, node.clone()));
        current_match.set(initial_match);

        Self {
            ctx,
            history: History::new(initial),
            route_tree,
            current_match,
            pending_navigation,
            blockers: HashMap::new(),
            events: Vec::new(),
            next_blocker_id: 0,
            not_found_mode: options.not_found_mode,
            not_found_data,
            default_pending_ms: options.default_pending_ms,
            default_pending_min_ms: options.default_pending_min_ms,
        }
    }

    pub fn navigate(&self, loc: Location, options: NavigateOptions) {
        // Implementation: check blockers, match route, update signals.
        // Use self.ctx.executor().spawn for async blocker checks.
        // After resolution, call commit_navigation.
    }

    fn commit_navigation(&self, loc: Location, options: NavigateOptions) {
        // Update current_match signal, history, emit events.
        // Trigger loaders via cache.
    }

    pub fn current_location(&self) -> Location {
        self.history.current()
    }

    // ... other methods (add_blocker, remove_blocker, proceed, etc.)
}

#[derive(Clone)]
pub struct RouterOptions {
    pub default_pending_ms: u64,
    pub default_pending_min_ms: u64,
    pub not_found_mode: NotFoundMode,
}

impl Default for RouterOptions {
    fn default() -> Self {
        Self {
            default_pending_ms: 1000,
            default_pending_min_ms: 500,
            not_found_mode: NotFoundMode::Root,
        }
    }
}
```

- [ ] **Step 2: Implement `navigate` with async blocker support**

```rust
pub fn navigate(&self, loc: Location, options: NavigateOptions) {
    log::debug!("navigate called: {}", loc.pathname);

    if !options.ignore_blocker && !self.blockers.is_empty() {
        let current = self.current_location();
        let blockers: Vec<Blocker> = self.blockers.values().cloned().collect();
        let loc_clone = loc.clone();
        let options_clone = options.clone();
        let ctx = self.ctx.clone();
        let router = self.clone(); // Router is Clone

        self.ctx.executor().spawn(async move {
            let mut futures = Vec::new();
            for blocker in &blockers {
                futures.push(blocker.should_allow(&current, &loc_clone));
            }
            let results = futures::future::join_all(futures).await;
            if results.iter().all(|&allow| allow) {
                router.commit_navigation(loc_clone, options_clone);
            } else {
                router.pending_navigation.set(Some(loc_clone));
            }
        });
        return;
    }

    self.commit_navigation(loc, options);
}
```

- [ ] **Step 3: Implement `commit_navigation`**

```rust
fn commit_navigation(&self, loc: Location, options: NavigateOptions) {
    // Update history
    if options.replace {
        self.history.replace(loc.clone());
    } else {
        self.history.push(loc.clone());
    }

    // Update current match
    let new_match = self.route_tree
        .match_path(&loc.pathname)
        .map(|(params, node)| (params, node.clone()));
    self.current_match.set(new_match);

    // Emit events
    self.emit(RouterEvent::Resolved {
        from: None,
        to: loc.clone(),
    });

    // Trigger loaders (will be added)
    self.ctx.request_update();
}
```

- [ ] **Step 4: Add remaining public methods**

Add `add_blocker`, `remove_blocker`, `proceed`, `reset_block`, `is_blocked`, `current_match`, `subscribe`, `emit`.

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p navi-quoin
```

Expected: Compiles (may have warnings about unused fields).

- [ ] **Step 6: Commit**

```bash
git add navi-quoin/src/core/state.rs
git commit -m "feat(navi-quoin): implement generic Router<C>"
```

### Task 2.2: Implement `Navigator<C>`

**Files:**
- Create: `navi-quoin/src/core/navigator.rs`

- [ ] **Step 1: Define `Navigator` struct**

```rust
// navi-quoin/src/core/navigator.rs
use quoin::ReactiveContext;
use crate::core::{Location, NavigateOptions, Router};

pub struct Navigator<C: ReactiveContext> {
    ctx: C,
    base: Option<String>,
}

impl<C: ReactiveContext> Navigator<C> {
    pub fn new(ctx: C) -> Self {
        Self { ctx, base: None }
    }

    pub fn from_route(ctx: C, base: impl Into<String>) -> Self {
        Self {
            ctx,
            base: Some(base.into()),
        }
    }

    pub fn push(&self, path: &str) {
        let loc = self.to_location(path);
        self.push_location(loc, NavigateOptions::default());
    }

    pub fn push_location(&self, loc: Location, options: NavigateOptions) {
        // Access global Router via context
        if let Some(router) = self.ctx.try_global::<Router<C>>() {
            router.navigate(loc, options);
        }
    }

    pub fn replace(&self, path: &str) {
        let loc = self.to_location(path);
        self.replace_location(loc);
    }

    pub fn replace_location(&self, loc: Location) {
        if let Some(router) = self.ctx.try_global::<Router<C>>() {
            router.navigate(loc, NavigateOptions { replace: true, ..Default::default() });
        }
    }

    fn to_location(&self, path: &str) -> Location {
        let resolved = if path.starts_with('/') {
            path.to_string()
        } else if let Some(base) = &self.base {
            format!("{}/{}", base.trim_end_matches('/'), path)
        } else {
            path.to_string()
        };
        Location::new(&resolved)
    }
}
```

- [ ] **Step 2: Add `try_global` method to `ReactiveContext` trait (if not present)**

Since `quoin::ReactiveContext` may not have `try_global`, we'll add a simple extension trait or use a global registry. For now, assume we'll store the `Router` in the context's associated global storage (to be implemented in glue).

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p navi-quoin
```

Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add navi-quoin/src/core/navigator.rs
git commit -m "feat(navi-quoin): implement generic Navigator<C>"
```

### Task 2.3: Implement `EventBus` Using Signals

**Files:**
- Create: `navi-quoin/src/core/event_bus.rs`

- [ ] **Step 1: Define event bus with a signal of events**

```rust
// navi-quoin/src/core/event_bus.rs
use quoin::{ReactiveContext, Signal};
use crate::core::RouterEvent;
use std::sync::Arc;

pub struct EventBus<C: ReactiveContext> {
    events: C::Signal<Vec<RouterEvent>>,
}

impl<C: ReactiveContext> EventBus<C> {
    pub fn new(ctx: &C) -> Self {
        Self {
            events: ctx.create_signal(Vec::new()),
        }
    }

    pub fn push(&self, event: RouterEvent) {
        self.events.update(|events| events.push(event));
    }

    pub fn subscribe(&self) -> C::Signal<Vec<RouterEvent>> {
        self.events.clone()
    }

    pub fn clear(&self) {
        self.events.set(Vec::new());
    }
}
```

- [ ] **Step 2: Integrate with `Router`**

In `Router::emit`, use the event bus to push events.

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p navi-quoin
```

- [ ] **Step 4: Commit**

```bash
git add navi-quoin/src/core/event_bus.rs
git commit -m "feat(navi-quoin): implement EventBus with signals"
```

---

## Chunk 3: Framework‑Agnostic UI Components

### Task 3.1: Implement `Link` Component

**Files:**
- Create: `navi-quoin/src/components/link.rs`
- Modify: `navi-quoin/src/components/mod.rs`

- [ ] **Step 1: Write `Link` component using `component!` macro**

```rust
// navi-quoin/src/components/link.rs
use quoin_macros::component;
use crate::core::Navigator;

component! {
    Link {
        props {
            to: String,
            replace: bool = false,
            class: String = "",
        }
        render {
            let nav = Navigator::new(ctx.clone());
            let onclick = action!(nav, to, replace => {
                if replace {
                    nav.replace(&to);
                } else {
                    nav.push(&to);
                }
            });
            quoin_render! {
                <div class={format!("cursor-pointer {}", class)} on_click={onclick}>
                    {children}
                </div>
            }
        }
    }
}
```

- [ ] **Step 2: Create `components/mod.rs`**

```rust
// navi-quoin/src/components/mod.rs
pub mod link;
pub mod outlet;
pub mod router_provider;
pub mod awaited;
pub mod scroll_restoration;
pub mod suspense_boundary;

pub use link::Link;
pub use outlet::Outlet;
pub use router_provider::RouterProvider;
pub use awaited::Awaited;
pub use scroll_restoration::ScrollRestoration;
pub use suspense_boundary::SuspenseBoundary;
```

- [ ] **Step 3: Verify compilation with GPUI feature**

```bash
cargo check -p navi-quoin --features gpui
```

Expected: `quoin-macros` expands to valid GPUI code.

- [ ] **Step 4: Commit**

```bash
git add navi-quoin/src/components/
git commit -m "feat(navi-quoin): implement Link component with quoin macros"
```

### Task 3.2: Implement `Outlet` Component

**Files:**
- Create: `navi-quoin/src/components/outlet.rs`

- [ ] **Step 1: Write `Outlet` component**

```rust
// navi-quoin/src/components/outlet.rs
use quoin_macros::component;
use crate::core::Router;

component! {
    Outlet {
        render {
            let router = ctx.try_global::<Router<_>>().expect("Router not found in context");
            let matched = router.current_match.get();
            quoin_render! {
                <div>
                    {matched.map(|(_, node)| {
                        // Render registered component for node.id
                        // Component registry will be accessed via context
                        render_route_component(node.id)
                    })}
                </div>
            }
        }
    }
}
```

- [ ] **Step 2: Define component registry**

We need a way to register route components. This will be done via a global registry in the context (similar to `register_route_component` in GPUI). We'll define a trait for it later.

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p navi-quoin --features gpui
```

- [ ] **Step 4: Commit**

```bash
git add navi-quoin/src/components/outlet.rs
git commit -m "feat(navi-quoin): implement Outlet component"
```

### Task 3.3: Implement `RouterProvider` Component

**Files:**
- Create: `navi-quoin/src/components/router_provider.rs`

- [ ] **Step 1: Write `RouterProvider` component**

```rust
// navi-quoin/src/components/router_provider.rs
use quoin_macros::component;
use crate::core::{Router, RouterOptions, Location, RouteTree};
use std::rc::Rc;

component! {
    RouterProvider {
        props {
            initial_location: Location,
            route_tree: Rc<RouteTree>,
            options: RouterOptions = RouterOptions::default(),
        }
        render {
            // Initialize router and store in context
            let router = Router::new(ctx.clone(), initial_location, route_tree, options);
            ctx.provide(router);
            quoin_render! {
                <Outlet />
            }
        }
    }
}
```

- [ ] **Step 2: Add `provide` method to context trait**

We'll need to extend `ReactiveContext` with a global storage mechanism. This will be implemented in framework glue.

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p navi-quoin --features gpui
```

- [ ] **Step 4: Commit**

```bash
git add navi-quoin/src/components/router_provider.rs
git commit -m "feat(navi-quoin): implement RouterProvider component"
```

### Task 3.4: Implement Remaining Components

- [ ] Implement `Awaited`, `ScrollRestoration`, `SuspenseBoundary` similarly using `component!` and `quoin_render!`. Each is a simple wrapper around existing logic.

- [ ] Commit each component separately.

---

## Chunk 4: Minimal Framework Glue (GPUI)

### Task 4.1: GPUI Global Storage for Router

**Files:**
- Create: `navi-quoin/src/glue/gpui.rs`
- Modify: `navi-quoin/src/lib.rs` (conditional export)

- [ ] **Step 1: Implement GPUI global storage for `Router`**

```rust
// navi-quoin/src/glue/gpui.rs
use gpui::{App, Global};
use quoin_gpui::GpuiContext;
use crate::core::Router;

pub struct GpuiRouter(pub Router<GpuiContext>);

impl Global for GpuiRouter {}

pub fn init_router(cx: &mut App, router: Router<GpuiContext>) {
    cx.set_global(GpuiRouter(router));
}

pub fn with_router<F, R>(cx: &App, f: F) -> R
where
    F: FnOnce(&Router<GpuiContext>) -> R,
{
    let global = cx.global::<GpuiRouter>();
    f(&global.0)
}
```

- [ ] **Step 2: Export glue module conditionally**

In `navi-quoin/src/lib.rs`:

```rust
#[cfg(feature = "gpui")]
pub mod glue;
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p navi-quoin --features gpui
```

- [ ] **Step 4: Commit**

```bash
git add navi-quoin/src/glue/ navi-quoin/src/lib.rs
git commit -m "feat(navi-quoin): add GPUI glue for global router storage"
```

### Task 4.2: Component Registration Glue

- [ ] Implement a registry for route components that works with GPUI's `register_route_component`. This can be a simple static `HashMap` behind a `Mutex`.

- [ ] Update `Outlet` to use this registry.

---

## Chunk 5: `navi-router` Re‑export Shim

### Task 5.1: Convert `navi-router` to Shim

**Files:**
- Modify: `navi-router/Cargo.toml`
- Replace: `navi-router/src/lib.rs`
- Delete: All old source files in `navi-router/src/`

- [ ] **Step 1: Update `navi-router/Cargo.toml`**

```toml
[package]
name = "navi-router"
version = "0.1.0"
edition = "2024"

[dependencies]
navi-quoin = { path = "../navi-quoin" }
navi-core = { path = "../navi-core" }

[features]
default = ["gpui"]
gpui = ["navi-quoin/gpui"]
leptos = ["navi-quoin/leptos"]
dioxus = ["navi-quoin/dioxus"]
nexum = []  # will forward later
```

- [ ] **Step 2: Write new `lib.rs`**

```rust
// navi-router/src/lib.rs
pub use navi_core::*;
pub use navi_quoin::core::*;
pub use navi_quoin::components::*;

#[cfg(feature = "gpui")]
pub use navi_quoin::glue::gpui::*;

#[cfg(feature = "gpui")]
pub type RouterState = navi_quoin::glue::gpui::GpuiRouter;
```

- [ ] **Step 3: Delete old source files**

```bash
rm -rf navi-router/src/*.rs navi-router/src/components/
```

- [ ] **Step 4: Verify workspace compilation**

```bash
cargo check --all
```

- [ ] **Step 5: Commit**

```bash
git add navi-router/
git commit -m "refactor(navi-router): convert to re-export shim for navi-quoin"
```

---

## Chunk 6: Update Macros and Codegen

### Task 6.1: Verify `navi-macros` Compatibility

- [ ] **Step 1: Ensure `define_route!` expands to use the shimmed `RouterState`.**

The macro already uses `RouterState::global(cx)`. After the shim, this resolves to `GpuiRouter::global(cx)`, which works.

- [ ] **Step 2: Run macro expansion test**

```bash
cargo expand -p example-app --example main
```

Expected: Expanded code uses `navi_router::RouterState`.

- [ ] **Step 3: Commit any necessary tweaks (if any)**

### Task 6.2: Verify `navi-codegen` Output

- [ ] **Step 1: Run codegen for `example-app`**

```bash
cd example-app && cargo build
```

Expected: Generated `route_tree.gen.rs` compiles with new shim.

- [ ] **Step 2: Commit (if changes were needed)**

---

## Chunk 7: Integration Testing with `example-app`

### Task 7.1: Update `example-app` Dependencies

**Files:**
- Modify: `example-app/Cargo.toml`

- [ ] **Step 1: Change dependency to use `navi-router` with `gpui` feature**

```toml
navi-router = { path = "../navi-router", features = ["gpui"] }
```

- [ ] **Step 2: Remove any direct `rs-query` dependency (now re-exported)**

- [ ] **Step 3: Build and run**

```bash
cargo run -p example-app
```

- [ ] **Step 4: Test all routes manually**

Verify that navigation, loaders, blockers, devtools all function as before.

- [ ] **Step 5: Commit**

```bash
git add example-app/Cargo.toml
git commit -m "chore(example-app): update to use navi-quoin shim"
```

---

## Plan Complete

The plan is now fully detailed and ready for execution. Each chunk is self-contained with explicit file paths, code snippets, and verification steps.

**Next Steps:** Execute chunk by chunk using the superpowers:executing-plans skill or subagent-driven development. After each chunk, run the reviewer and proceed only when approved.
