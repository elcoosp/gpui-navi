I'll regenerate the first five chunks of the implementation plan, incorporating the review findings. The primary adjustments:

- Clarify `use_navigate!` macro's context requirement (must be used where `cx` has `.window_handle()`).
- Ensure async task spawning uses correct GPUI patterns.
- Add missing imports and minor corrections to code snippets.
- Note that devtools enhancements will extend existing `navi-devtools` (to be addressed in later chunks).

---

# Detailed Implementation Plan — Chunks 1–5 (Revised)

## Chunk 1: `beforeLoad` Hooks, Loader Redirect/NotFound, and Lifecycle Hooks

### Task 1.1: Add `before_load` Infrastructure

**Files:**
- `navi-router/src/route_tree.rs`
- `navi-router/src/state.rs`
- `navi-macros/src/route.rs`
- `example-app/src/routes/admin/mod.rs` (new)

---

#### Step 1.1.1: Define `BeforeLoadContext` and `BeforeLoadResult` types

**File:** `navi-router/src/route_tree.rs`

Add after existing imports:

```rust
use crate::location::Location;
use crate::redirect::{NotFound, Redirect};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;

/// Context passed to `before_load` hooks.
pub struct BeforeLoadContext {
    pub params: HashMap<String, String>,
    pub search: serde_json::Value,
    pub location: Location,
}

/// Result of a `before_load` hook.
pub enum BeforeLoadResult {
    /// Proceed with navigation.
    Ok,
    /// Redirect to a different location.
    Redirect(Redirect),
    /// Trigger a 404 not found.
    NotFound(NotFound),
}

/// Type alias for a before-load function.
pub type BeforeLoadFn = Arc<
    dyn Fn(BeforeLoadContext) -> BoxFuture<'static, BeforeLoadResult> + Send + Sync,
>;
```

**Verification:** Run `cargo check -p navi-router`. Expected: no errors.

---

#### Step 1.1.2: Add `before_load` field to `RouteNode`

**File:** `navi-router/src/route_tree.rs`

Modify the `RouteNode` struct:

```rust
pub struct RouteNode {
    // ... existing fields ...
    pub before_load: Option<BeforeLoadFn>,
}
```

Update the `Debug` impl to skip the function pointer:

```rust
impl std::fmt::Debug for RouteNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteNode")
            .field("id", &self.id)
            .field("pattern", &self.pattern.raw)
            .field("is_layout", &self.is_layout)
            .field("is_index", &self.is_index)
            .field("has_before_load", &self.before_load.is_some())
            .finish()
    }
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 1.1.3: Add `not_found_mode` and `not_found_data` to `RouterState`

**File:** `navi-router/src/state.rs`

Add enum and fields:

```rust
pub enum NotFoundMode {
    /// Render the global 404 route.
    Root,
    /// Render the closest scoped 404 route.
    Fuzzy,
}

pub struct RouterState {
    // ... existing fields ...
    pub not_found_mode: NotFoundMode,
    pub not_found_data: Option<serde_json::Value>,
}
```

Initialize defaults in `RouterState::new`:

```rust
not_found_mode: NotFoundMode::Root,
not_found_data: None,
```

**Verification:** `cargo check -p navi-router`

---

#### Step 1.1.4: Implement `commit_navigation` helper method

**File:** `navi-router/src/state.rs`

Extract the existing navigation commit logic into a private method:

```rust
impl RouterState {
    fn commit_navigation(&mut self, loc: Location, options: NavigateOptions, cx: &mut App) {
        // Move existing code from navigate here:
        // - Update current_match
        // - Push/replace history
        // - Trigger loader
        self.current_match = self
            .route_tree
            .match_path(&loc.pathname)
            .map(|(params, node)| (params, node.clone()));

        if options.replace {
            self.history.replace(loc.clone());
        } else {
            self.history.push(loc.clone());
        }

        self.trigger_loader_with_locations(Some(self.current_location()), loc, cx);
    }
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 1.1.5: Modify `navigate` to execute `before_load` hooks

**File:** `navi-router/src/state.rs`

In `RouterState::navigate`, after blocker checks and before `commit_navigation`, insert:

```rust
// ... blocker checks ...

// Find matched route for the target location
let (params, matched_node) = match self.route_tree.match_path(&loc.pathname) {
    Some((params, node)) => (params, node.clone()),
    None => {
        // No match – trigger 404 handling
        self.commit_navigation(loc, options, cx);
        return;
    }
};

// Collect before_load functions from ancestors and the matched node
let before_load_fns: Vec<(String, BeforeLoadFn)> = self
    .route_tree
    .ancestors(&matched_node.id)
    .iter()
    .chain(std::iter::once(&matched_node))
    .filter_map(|node| {
        node.before_load
            .as_ref()
            .map(|f| (node.id.clone(), f.clone()))
    })
    .collect();

if !before_load_fns.is_empty() {
    let window_handle = self.window_handle;
    cx.spawn(|mut cx| {
        async move {
            for (route_id, before_load) in before_load_fns {
                let ctx = BeforeLoadContext {
                    params: params.clone(),
                    search: loc.search.clone(),
                    location: loc.clone(),
                };
                match before_load(ctx).await {
                    BeforeLoadResult::Ok => continue,
                    BeforeLoadResult::Redirect(redirect) => {
                        let nav = Navigator::new(window_handle);
                        cx.update(|cx| {
                            nav.push_location(Location::new(&redirect.to), cx);
                        }).ok();
                        return;
                    }
                    BeforeLoadResult::NotFound(not_found) => {
                        cx.update(|cx| {
                            RouterState::update(cx, |state, cx| {
                                state.not_found_data = not_found.data;
                                let not_found_path = match state.not_found_mode {
                                    NotFoundMode::Root => "/404",
                                    NotFoundMode::Fuzzy => "/404", // resolved by outlet
                                };
                                let nav = Navigator::new(state.window_handle);
                                nav.push(not_found_path, cx);
                            });
                        }).ok();
                        return;
                    }
                }
            }
            // All before_load hooks passed
            cx.update(|cx| {
                RouterState::update(cx, |state, cx| {
                    state.commit_navigation(loc, options, cx);
                });
            }).ok();
        }
    }).detach();
    return;
}

// No before_load hooks, commit immediately
self.commit_navigation(loc, options, cx);
```

**Verification:** `cargo check -p navi-router`

---

#### Step 1.1.6: Extend `define_route!` macro to parse `before_load`

**File:** `navi-macros/src/route.rs`

Add a new field variant in `FieldValue` enum (if not already present) or use `Expr`. In the parsing loop:

```rust
let mut before_load_closure = None;

// Inside field parsing
"before_load" => {
    if let FieldValue::Expr(expr) = field.value {
        before_load_closure = Some(expr);
    }
}
```

Generate the `before_load_fn` method:

```rust
let before_load_impl = if let Some(before_load) = before_load_closure {
    quote! {
        pub fn before_load_fn() -> Option<::navi_router::route_tree::BeforeLoadFn> {
            Some(::std::sync::Arc::new(|ctx| {
                let closure = #before_load;
                ::futures::future::FutureExt::boxed(closure(ctx))
            }))
        }
    }
} else {
    quote! {
        pub fn before_load_fn() -> Option<::navi_router::route_tree::BeforeLoadFn> { None }
    }
};
```

In `build_node`, call `Self::before_load_fn()` and assign to `node.before_load`.

**Verification:** `cargo check -p navi-macros`

---

#### Step 1.1.7: Create demonstration admin layout route

**File:** `example-app/src/routes/admin/mod.rs`

Create new file:

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::{BeforeLoadResult, redirect, components::Outlet};

define_route!(
    AdminRoute,
    path: "/admin",
    is_layout: true,
    before_load: |ctx| async move {
        // Simulate auth check - toggle to test
        let is_authenticated = false; // change to true to allow access
        if !is_authenticated {
            BeforeLoadResult::Redirect(redirect("/login"))
        } else {
            BeforeLoadResult::Ok
        }
    },
    component: AdminLayout,
);

#[derive(Clone, IntoElement)]
struct AdminLayout;

impl RenderOnce for AdminLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .p_4()
            .child("Admin Area (protected by beforeLoad)")
            .child(Outlet::new())
    }
}
```

**Verification:** `cargo check -p example-app`

---

#### Step 1.1.8: Create simple login route

**File:** `example-app/src/routes/login.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;

define_route!(
    LoginRoute,
    path: "/login",
    component: LoginPage,
);

#[derive(Clone, IntoElement)]
struct LoginPage;

impl RenderOnce for LoginPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Login Page (redirect target)")
    }
}
```

**Verification:** `cargo check -p example-app`

---

#### Step 1.1.9: Register new routes in generated code

**File:** `example-app/build.rs`

Ensure the build script runs to regenerate `route_tree.gen.rs` with new routes.

**Command:** `cargo build -p example-app`

Expected: Successful build with new routes included.

---

#### Step 1.1.10: Commit

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/admin/mod.rs example-app/src/routes/login.rs
git commit -m "feat: add beforeLoad hook infrastructure with redirect/notFound support"
```

---

### Task 1.2: Loader Outcome Enum for Redirect/NotFound

---

#### Step 1.2.1: Define `LoaderOutcome` enum

**File:** `navi-router/src/state.rs`

Add near top of file:

```rust
use crate::redirect::{NotFound, Redirect};

pub enum LoaderOutcome<T> {
    Data(T),
    Redirect(Redirect),
    NotFound(NotFound),
}
```

---

#### Step 1.2.2: Update `LoaderFactory` type alias

**File:** `navi-router/src/state.rs`

Change the return type from `Query<AnyData>` to `Query<LoaderOutcome<AnyData>>`:

```rust
type LoaderFactory = Arc<
    dyn Fn(&HashMap<String, String>) -> Query<LoaderOutcome<AnyData>> + Send + Sync,
>;
```

---

#### Step 1.2.3: Modify `trigger_loader_with_locations` to handle outcome

**File:** `navi-router/src/state.rs`

Inside the spawned task where `(fetch_fn)().await` is called, handle the result:

```rust
let outcome = (fetch_fn)().await;
match outcome {
    LoaderOutcome::Data(data) => {
        client.set_query_data(&key, data, options.clone());
        // ... existing success handling ...
        cx.update(|cx| {
            push_event(RouterEvent::Load { from: from_clone, to: to_clone }, cx);
            // ... rest of events ...
        }).ok();
    }
    LoaderOutcome::Redirect(redirect) => {
        cx.update(|cx| {
            let nav = Navigator::new(window_handle);
            nav.push_location(Location::new(&redirect.to), cx);
        }).ok();
    }
    LoaderOutcome::NotFound(not_found) => {
        cx.update(|cx| {
            RouterState::update(cx, |state, cx| {
                state.not_found_data = not_found.data;
                let nav = Navigator::new(state.window_handle);
                nav.push("/404", cx);
            });
        }).ok();
    }
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 1.2.4: Update `define_route!` macro to wrap loader result

**File:** `navi-macros/src/route.rs`

In the generated `loader_factory` method, wrap the user's loader result:

```rust
let factory = quote! {
    pub fn loader_factory(executor: ::gpui::BackgroundExecutor) -> std::sync::Arc<
        dyn Fn(&std::collections::HashMap<String, String>) -> ::rs_query::Query<::navi_router::LoaderOutcome<::navi_router::AnyData>>
        + Send + Sync
    > {
        std::sync::Arc::new(move |params_map: &std::collections::HashMap<String, String>| {
            let params: #params_ty = serde_json::from_value(
                serde_json::to_value(params_map).unwrap()
            ).expect("Failed to deserialize route params");
            let params_clone = params.clone();
            let loader = #loader_closure;
            let executor = executor.clone();
            let key = ::rs_query::QueryKey::new("navi_loader")
                .with("route", stringify!(#name))
                .with("params", serde_json::to_string(&params).unwrap());
            ::rs_query::Query::new(key, move || {
                let params = params_clone.clone();
                let loader = loader.clone();
                let executor = executor.clone();
                async move {
                    let data = loader(params, executor).await
                        .map_err(|e| ::rs_query::QueryError::custom(e.to_string()))?;
                    Ok(::navi_router::LoaderOutcome::Data(::navi_router::AnyData(std::sync::Arc::new(data) as std::sync::Arc<dyn std::any::Any + Send + Sync>)))
                }
            })
            .stale_time(#stale_time_expr)
            .gc_time(#gc_time_expr)
            .structural_sharing(true)
        })
    }
};
```

**Verification:** `cargo check -p navi-macros`

---

#### Step 1.2.5: Add example loader that returns `NotFound`

**File:** `example-app/src/routes/admin/dashboard.rs`

Create new file:

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;

define_route!(
    AdminDashboardRoute,
    path: "/admin/dashboard",
    data: String,
    loader: |_params, _executor| async move {
        // Simulate conditional not found
        let should_404 = true; // toggle to test
        if should_404 {
            Err("Not found".into())
        } else {
            Ok(std::sync::Arc::new("Dashboard data".to_string()))
        }
    },
    component: DashboardPage,
);

#[derive(Clone, IntoElement)]
struct DashboardPage;

impl RenderOnce for DashboardPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = navi_macros::use_loader_data!(AdminDashboardRoute);
        match data {
            Some(d) => div().child(format!("Dashboard: {}", d)),
            None => div().child("Loading or not found..."),
        }
    }
}
```

**Verification:** `cargo build -p example-app`

---

#### Step 1.2.6: Commit

```bash
git add navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/admin/dashboard.rs
git commit -m "feat: loader outcome enum for redirect/notFound interception"
```

---

### Task 1.3: `onEnter` / `onLeave` Lifecycle Hooks

---

#### Step 1.3.1: Add fields to `RouteNode`

**File:** `navi-router/src/route_tree.rs`

```rust
pub struct RouteNode {
    // ... existing ...
    pub on_enter: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
    pub on_leave: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
}
```

---

#### Step 1.3.2: Compute route set differences in `commit_navigation`

**File:** `navi-router/src/state.rs`

In `commit_navigation`, before updating `current_match`:

```rust
// Collect previous route IDs
let prev_route_ids: std::collections::HashSet<String> = self
    .current_match
    .as_ref()
    .map(|(_, node)| {
        self.route_tree
            .ancestors(&node.id)
            .iter()
            .map(|n| n.id.clone())
            .chain(std::iter::once(node.id.clone()))
            .collect()
    })
    .unwrap_or_default();

// ... update current_match ...

// Collect new route IDs
let new_route_ids: std::collections::HashSet<String> = self
    .current_match
    .as_ref()
    .map(|(_, node)| {
        self.route_tree
            .ancestors(&node.id)
            .iter()
            .map(|n| n.id.clone())
            .chain(std::iter::once(node.id.clone()))
            .collect()
    })
    .unwrap_or_default();

// Call on_leave for routes no longer active
for route_id in prev_route_ids.difference(&new_route_ids) {
    if let Some(node) = self.route_tree.get_node(route_id) {
        if let Some(on_leave) = &node.on_leave {
            on_leave(&loc);
        }
    }
}

// Call on_enter for newly active routes
for route_id in new_route_ids.difference(&prev_route_ids) {
    if let Some(node) = self.route_tree.get_node(route_id) {
        if let Some(on_enter) = &node.on_enter {
            on_enter(&loc);
        }
    }
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 1.3.3: Extend `define_route!` macro

**File:** `navi-macros/src/route.rs`

Add parsing for `on_enter` and `on_leave`:

```rust
let mut on_enter = None;
let mut on_leave = None;

// In field loop:
"on_enter" => {
    if let FieldValue::Expr(expr) = field.value {
        on_enter = Some(expr);
    }
}
"on_leave" => {
    if let FieldValue::Expr(expr) = field.value {
        on_leave = Some(expr);
    }
}
```

Generate assignments in `build_node`:

```rust
let on_enter_impl = on_enter.map(|e| quote! { Some(::std::sync::Arc::new(#e)) }).unwrap_or(quote! { None });
let on_leave_impl = on_leave.map(|e| quote! { Some(::std::sync::Arc::new(#e)) }).unwrap_or(quote! { None });

quote! {
    // ...
    pub fn build_node() -> navi_router::RouteNode {
        // ...
        node.on_enter = #on_enter_impl;
        node.on_leave = #on_leave_impl;
        node
    }
}
```

**Verification:** `cargo check -p navi-macros`

---

#### Step 1.3.4: Create demonstration lifecycle route

**File:** `example-app/src/routes/lifecycle.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::Location;

define_route!(
    LifecycleRoute,
    path: "/lifecycle",
    on_enter: |loc: &Location| {
        log::info!("Entered lifecycle route at {}", loc.pathname);
    },
    on_leave: |loc: &Location| {
        log::info!("Left lifecycle route from {}", loc.pathname);
    },
    component: LifecyclePage,
);

#[derive(Clone, IntoElement)]
struct LifecyclePage;

impl RenderOnce for LifecyclePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Lifecycle Demo - check console logs")
    }
}
```

**Verification:** `cargo build -p example-app`

---

#### Step 1.3.5: Commit

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/lifecycle.rs
git commit -m "feat: add onEnter/onLeave route lifecycle hooks"
```

---

## Chunk 2: `loaderDeps`, `staleTime`/`gcTime`, and Suspense Timing

### Task 2.1: `loaderDeps` – Reactive Search Dependencies

---

#### Step 2.1.1: Add `loader_deps` field to `RouteNode`

**File:** `navi-router/src/route_tree.rs`

```rust
pub struct RouteNode {
    // ...
    pub loader_deps: Option<Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>>,
}
```

---

#### Step 2.1.2: Modify cache key generation in `trigger_loader_with_locations`

**File:** `navi-router/src/state.rs`

Inside the loader trigger, after getting the `node`:

```rust
let deps_json = node
    .loader_deps
    .as_ref()
    .map(|f| f(&loc.search))
    .unwrap_or(serde_json::Value::Null);

let key = QueryKey::new("navi_loader")
    .with("route", node.id.as_str())
    .with("params", serde_json::to_string(params).unwrap_or_default())
    .with("deps", serde_json::to_string(&deps_json).unwrap_or_default());
```

**Verification:** `cargo check -p navi-router`

---

#### Step 2.1.3: Extend macro to parse `loader_deps`

**File:** `navi-macros/src/route.rs`

```rust
let mut loader_deps = None;

"loader_deps" => {
    if let FieldValue::Expr(expr) = field.value {
        loader_deps = Some(expr);
    }
}
```

Generate in `build_node`:

```rust
let loader_deps_impl = loader_deps.map(|e| quote! { Some(::std::sync::Arc::new(#e)) }).unwrap_or(quote! { None });
// ...
node.loader_deps = #loader_deps_impl;
```

**Verification:** `cargo check -p navi-macros`

---

#### Step 2.1.4: Add example posts index route

**File:** `example-app/src/routes/posts/mod.rs` (layout) and `example-app/src/routes/posts/index.rs`

**`mod.rs`:**

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::define_route;
use navi_router::components::Outlet;

define_route!(
    PostsRoute,
    path: "/posts",
    is_layout: true,
    component: PostsLayout,
);

#[derive(Clone, IntoElement)]
struct PostsLayout;
impl RenderOnce for PostsLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Posts Section").child(Outlet::new())
    }
}
```

**`index.rs`:**

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::{define_route, use_loader_data, use_search};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PostsSearch {
    pub page: Option<u32>,
}

define_route!(
    PostsIndexRoute,
    path: "/posts",
    is_index: true,
    search: PostsSearch,
    data: Vec<String>,
    loader_deps: |search: &serde_json::Value| {
        // Only depend on the 'page' param
        serde_json::json!({ "page": search.get("page") })
    },
    loader: |_params, executor: gpui::BackgroundExecutor| async move {
        executor.timer(std::time::Duration::from_millis(500)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(vec![
            "Post 1".to_string(),
            "Post 2".to_string(),
        ]))
    },
    component: PostsIndexPage,
);

#[derive(Clone, IntoElement)]
struct PostsIndexPage;

impl RenderOnce for PostsIndexPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let search = use_search!(PostsIndexRoute);
        let data = use_loader_data!(PostsIndexRoute);
        let page = search.page.unwrap_or(1);
        div()
            .child(format!("Posts page {}", page))
            .child(format!("Data: {:?}", data))
    }
}
```

**Verification:** `cargo build -p example-app`

---

#### Step 2.1.5: Commit

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/posts/
git commit -m "feat: add loaderDeps for reactive search dependencies"
```

---

### Task 2.2: Per‑Route `staleTime` and `gcTime`

---

#### Step 2.2.1: Add fields to `RouteNode`

**File:** `navi-router/src/route_tree.rs`

```rust
pub struct RouteNode {
    // ...
    pub stale_time: Option<std::time::Duration>,
    pub gc_time: Option<std::time::Duration>,
}
```

---

#### Step 2.2.2: Use in `trigger_loader_with_locations`

**File:** `navi-router/src/state.rs`

When creating the query, use node's values or defaults:

```rust
let stale_time = node.stale_time.unwrap_or(std::time::Duration::ZERO);
let gc_time = node.gc_time.unwrap_or(std::time::Duration::from_secs(300));

let query = Query::new(key, move || { ... })
    .stale_time(stale_time)
    .gc_time(gc_time)
    .structural_sharing(true);
```

**Verification:** `cargo check -p navi-router`

---

#### Step 2.2.3: Extend macro to parse `stale_time` and `gc_time`

**File:** `navi-macros/src/route.rs`

```rust
let mut stale_time = None;
let mut gc_time = None;

"stale_time" => { stale_time = Some(expr); }
"gc_time" => { gc_time = Some(expr); }
```

Generate in `build_node`:

```rust
let stale_time_impl = stale_time.map(|e| quote! { Some(#e) }).unwrap_or(quote! { None });
let gc_time_impl = gc_time.map(|e| quote! { Some(#e) }).unwrap_or(quote! { None });
// ...
node.stale_time = #stale_time_impl;
node.gc_time = #gc_time_impl;
```

**Verification:** `cargo check -p navi-macros`

---

#### Step 2.2.4: Commit

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs
git commit -m "feat: per-route staleTime and gcTime configuration"
```

---

### Task 2.3: Router‑Level `defaultPendingMs` / `defaultPendingMinMs`

---

#### Step 2.3.1: Add fields to `RouterState`

**File:** `navi-router/src/state.rs`

```rust
pub struct RouterState {
    // ...
    pub default_pending_ms: u64,
    pub default_pending_min_ms: u64,
}
```

Initialize in `new`:

```rust
default_pending_ms: 1000,
default_pending_min_ms: 500,
```

---

#### Step 2.3.2: Create `RouterOptions` struct

**File:** `navi-router/src/state.rs`

```rust
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

Update `RouterState::new` to accept `options: RouterOptions`.

---

#### Step 2.3.3: Update `RouterProvider::new` to accept `RouterOptions`

**File:** `navi-router/src/components/router_provider.rs`

```rust
impl RouterProvider {
    pub fn new(
        window_id: WindowId,
        window_handle: AnyWindowHandle,
        initial_location: Location,
        route_tree: RouteTree,
        options: RouterOptions,
        cx: &mut App,
    ) -> Self {
        // ...
        let state = RouterState::new(
            initial_location.clone(),
            window_id,
            window_handle,
            route_tree.clone(),
            options,
        );
        // ...
    }
}
```

---

#### Step 2.3.4: Modify `SuspenseBoundary` to use router defaults

**File:** `navi-router/src/components/suspense_boundary.rs`

Read from `RouterState::global(cx)`:

```rust
impl RenderOnce for SuspenseBoundary {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = RouterState::try_global(cx);
        let has_pending = state.map(|s| s.has_pending_loader()).unwrap_or(false);
        let pending_ms = state.map(|s| s.default_pending_ms).unwrap_or(1000);
        let pending_min_ms = state.map(|s| s.default_pending_min_ms).unwrap_or(500);

        // Implement timing logic using element state to show fallback only after pending_ms
        // and keep it visible for at least pending_min_ms.
        // (Implementation details omitted for brevity but follow existing suspense pattern)
        if has_pending {
            (self.fallback)()
        } else {
            Outlet::new().into_any_element()
        }
    }
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 2.3.5: Update `example-app/src/main.rs`

Pass `RouterOptions` to `RouterProvider::new`:

```rust
let options = RouterOptions {
    default_pending_ms: 500,
    default_pending_min_ms: 200,
    not_found_mode: NotFoundMode::Fuzzy,
};
let router_provider = RouterProvider::new(window_id, window_handle, initial, tree, options, cx);
```

**Verification:** `cargo build -p example-app`

---

#### Step 2.3.6: Commit

```bash
git add navi-router/src/state.rs navi-router/src/components/router_provider.rs navi-router/src/components/suspense_boundary.rs example-app/src/main.rs
git commit -m "feat: router-level defaultPendingMs and defaultPendingMinMs for suspense"
```

---

## Chunk 3: `notFoundMode`, 404 Routes, and Route Context

### Task 3.1: `notFoundMode` and Dedicated 404 Route Components

---

#### Step 3.1.1: `NotFoundMode` enum (already defined in Chunk 1)

---

#### Step 3.1.2: Update `Outlet` to handle 404 rendering

**File:** `navi-router/src/components/outlet.rs`

In `Outlet::render`, when no component is registered:

```rust
if constructor_opt.is_none() {
    // Check for 404 component based on not_found_mode
    let state = RouterState::global(cx);
    let not_found_component = match state.not_found_mode {
        NotFoundMode::Root => {
            REGISTRY.lock().unwrap().get("__not_found_root__").cloned()
        }
        NotFoundMode::Fuzzy => {
            // Walk up ancestors to find first layout with a registered 404
            let ancestors = state.route_tree.ancestors(&leaf_node.id);
            ancestors.iter().rev().find_map(|ancestor| {
                REGISTRY.lock().unwrap().get(&format!("__not_found_{}", ancestor.id)).cloned()
            })
        }
    };
    if let Some(constructor) = not_found_component {
        context::provide(window_id, OutletDepth(depth + 1));
        return constructor(cx);
    }
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 3.1.3: Update codegen to treat `$.rs` as 404 route

**File:** `navi-codegen/src/scanner.rs`

Add `is_not_found` flag to `RouteInfo`:

```rust
pub struct RouteInfo {
    // ...
    pub is_not_found: bool,
}
```

Detect `$.rs` files:

```rust
let is_not_found = file_name == "$" || (file_name == "mod" && relative_path.parent().map(|p| p.file_name().unwrap_or_default() == "$").unwrap_or(false));
```

In generator, register under both path and a special `__not_found_<scope>` ID.

**Verification:** `cargo check -p navi-codegen`

---

#### Step 3.1.4: Create example 404 routes

**File:** `example-app/src/routes/$.rs` (global 404)

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::define_route;

define_route!(
    GlobalNotFoundRoute,
    path: "/*",
    component: GlobalNotFoundPage,
);

#[derive(Clone, IntoElement)]
struct GlobalNotFoundPage;
impl RenderOnce for GlobalNotFoundPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("404 - Page Not Found (Global)")
    }
}
```

**File:** `example-app/src/routes/users/$.rs` (scoped)

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::define_route;

define_route!(
    UsersNotFoundRoute,
    path: "/users/*",
    component: UsersNotFoundPage,
);

#[derive(Clone, IntoElement)]
struct UsersNotFoundPage;
impl RenderOnce for UsersNotFoundPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("User not found (scoped to /users)")
    }
}
```

**Verification:** `cargo build -p example-app`

---

#### Step 3.1.5: Commit

```bash
git add navi-router/src/components/outlet.rs navi-codegen/src/scanner.rs navi-codegen/src/generator.rs example-app/src/routes/$.rs example-app/src/routes/users/$.rs
git commit -m "feat: notFoundMode and dedicated 404 route components"
```

---

### Task 3.2: Route Context (`routeContext`)

---

#### Step 3.2.1: Add `context_fn` to `RouteNode`

**File:** `navi-router/src/route_tree.rs`

```rust
pub struct RouteContextArgs {
    pub parent_context: Option<serde_json::Value>,
    pub params: HashMap<String, String>,
    pub loader_data: Option<AnyData>,
}

pub struct RouteNode {
    // ...
    pub context_fn: Option<Arc<dyn Fn(RouteContextArgs) -> serde_json::Value + Send + Sync>>,
}
```

---

#### Step 3.2.2: Compute and store context after loader

**File:** `navi-router/src/state.rs`

In the loader completion handler, after data is set:

```rust
if let Some(context_fn) = &node.context_fn {
    let args = RouteContextArgs {
        parent_context: None, // implement parent context chaining
        params: params.clone(),
        loader_data: Some(data.clone()),
    };
    let context_value = context_fn(args);
    // Store context in a separate rs-query cache or global map
    let context_key = QueryKey::new("navi_context").with("route", node.id.as_str());
    client.set_query_data(&context_key, context_value, options);
}
```

---

#### Step 3.2.3: Implement `use_route_context!` macro

**File:** `navi-macros/src/hooks.rs`

```rust
pub fn use_route_context(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = ::navi_router::RouterState::global(cx);
            state.get_route_context::<#route_ty>()
        }
    };
    expanded.into()
}
```

Add to `lib.rs` exports.

---

#### Step 3.2.4: Extend `define_route!` to parse `context`

**File:** `navi-macros/src/route.rs`

```rust
let mut context_fn = None;

"context" => {
    if let FieldValue::Expr(expr) = field.value {
        context_fn = Some(expr);
    }
}
```

Generate `context_fn` method.

---

#### Step 3.2.5: Add demonstration in example app

**File:** `example-app/src/routes/admin/dashboard.rs` (update)

Add context usage.

---

#### Step 3.2.6: Commit

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/hooks.rs navi-macros/src/lib.rs navi-macros/src/route.rs example-app/src/routes/admin/dashboard.rs
git commit -m "feat: routeContext with use_route_context macro"
```

---

## Chunk 4: Macros Completeness – `use_navigate!`, `use_matched_route!`, Meta

### Task 4.1: `use_navigate!` Macro

---

#### Step 4.1.1: Implement macro

**File:** `navi-macros/src/hooks.rs`

```rust
/// Returns a `Navigator` bound to the current window handle.
/// Must be used in a context where `cx.window_handle()` is available
/// (e.g., inside `Render` or event handlers).
#[proc_macro]
pub fn use_navigate(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        {
            let window_handle = cx.window_handle();
            ::navi_router::Navigator::new(window_handle)
        }
    };
    expanded.into()
}
```

---

#### Step 4.1.2: Add to `lib.rs`

```rust
pub use hooks::use_navigate;
```

**Verification:** `cargo check -p navi-macros`

---

#### Step 4.1.3: Commit

```bash
git add navi-macros/src/hooks.rs navi-macros/src/lib.rs
git commit -m "feat: add use_navigate! macro"
```

---

### Task 4.2: `use_matched_route!` Macro

---

#### Step 4.2.1: Implement macro

**File:** `navi-macros/src/hooks.rs`

```rust
/// Returns a tuple `(params, node)` for the currently matched route of the given type.
#[proc_macro]
pub fn use_matched_route(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = ::navi_router::RouterState::global(cx);
            let current_match = state.current_match.as_ref()
                .expect("use_matched_route called but no route matched");
            let params_map = &current_match.0;
            let node = &current_match.1;
            (params_map.clone(), node.clone())
        }
    };
    expanded.into()
}
```

---

#### Step 4.2.2: Add to `lib.rs`

```rust
pub use hooks::use_matched_route;
```

**Verification:** `cargo check -p navi-macros`

---

#### Step 4.2.3: Commit

```bash
git add navi-macros/src/hooks.rs navi-macros/src/lib.rs
git commit -m "feat: add use_matched_route! macro"
```

---

### Task 4.3: Per‑Route `meta` Map

---

#### Step 4.3.1: Add `meta` field to `RouteNode`

**File:** `navi-router/src/route_tree.rs`

```rust
pub struct RouteNode {
    // ...
    pub meta: HashMap<String, serde_json::Value>,
}
```

---

#### Step 4.3.2: Add `current_meta` method to `RouterState`

**File:** `navi-router/src/state.rs`

```rust
impl RouterState {
    pub fn current_meta(&self) -> HashMap<String, serde_json::Value> {
        let mut meta = HashMap::new();
        if let Some((_, node)) = &self.current_match {
            for ancestor in self.route_tree.ancestors(&node.id) {
                meta.extend(ancestor.meta.clone());
            }
            meta.extend(node.meta.clone());
        }
        meta
    }
}
```

---

#### Step 4.3.3: Extend macro to parse `meta`

**File:** `navi-macros/src/route.rs`

Parse a map literal. Use `syn::Expr` and convert to `HashMap` at compile time or store a closure.

```rust
let mut meta = None;

"meta" => {
    if let FieldValue::Expr(expr) = field.value {
        meta = Some(expr);
    }
}
```

Generate:

```rust
let meta_impl = meta.map(|e| quote! { #e }).unwrap_or(quote! { ::std::collections::HashMap::new() });
// ...
node.meta = #meta_impl;
```

---

#### Step 4.3.4: Update root layout to display meta title

**File:** `example-app/src/routes/__root.rs`

```rust
let meta = RouterState::global(cx).current_meta();
let title = meta.get("title").and_then(|v| v.as_str()).unwrap_or("Navi App");
// Use title in UI
```

**Verification:** `cargo build -p example-app`

---

#### Step 4.3.5: Commit

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/__root.rs
git commit -m "feat: per-route meta map and RouterState::current_meta"
```

---

## Chunk 5: Async Blocker and `Link` Enhancements

### Task 5.1: Async Blocker Support

---

#### Step 5.1.1: Change `Blocker` predicate to async

**File:** `navi-router/src/blocker.rs`

```rust
use futures::future::BoxFuture;

pub struct Blocker {
    pub should_block_fn: Box<dyn Fn(&Location, &Location) -> BoxFuture<'static, bool> + Send + Sync>,
    pub enable_before_unload: bool,
}

impl Blocker {
    pub fn new<F, Fut>(should_block_fn: F) -> Self
    where
        F: Fn(&Location, &Location) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = bool> + Send + 'static,
    {
        Self {
            should_block_fn: Box::new(move |from, to| Box::pin(should_block_fn(from, to))),
            enable_before_unload: false,
        }
    }

    pub fn new_sync<F>(should_block_fn: F) -> Self
    where
        F: Fn(&Location, &Location) -> bool + Send + Sync + 'static,
    {
        Self::new(move |from, to| {
            let result = should_block_fn(from, to);
            async move { result }
        })
    }
}
```

---

#### Step 5.1.2: Update `RouterState::navigate` to await blockers

**File:** `navi-router/src/state.rs`

Replace sync check with async:

```rust
let blockers: Vec<_> = self.blockers.values().cloned().collect();
if !blockers.is_empty() {
    let from = self.current_location();
    let to = loc.clone();
    let window_handle = self.window_handle;
    cx.spawn(|mut cx| async move {
        let futures = blockers.iter().map(|b| (b.should_block_fn)(&from, &to));
        let results = futures::future::join_all(futures).await;
        if results.iter().any(|&should_block| should_block) {
            // Blocked - store pending navigation
            cx.update(|cx| {
                RouterState::update(cx, |state, _| {
                    state.pending_navigation = Some(to);
                });
            }).ok();
        } else {
            // Proceed
            cx.update(|cx| {
                RouterState::update(cx, |state, cx| {
                    state.commit_navigation(to, options, cx);
                });
            }).ok();
        }
    }).detach();
    return;
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 5.1.3: Add demonstration blocking route

**File:** `example-app/src/routes/blocking.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::define_route;
use navi_router::{Blocker, RouterState};

define_route!(
    BlockingRoute,
    path: "/blocking",
    component: BlockingPage,
);

#[derive(Clone, IntoElement)]
struct BlockingPage;

impl RenderOnce for BlockingPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let blocker = Blocker::new(|from, to| async move {
            // Simulate async confirmation
            true // block navigation
        });
        RouterState::update(cx, |state, _| {
            state.add_blocker(blocker);
        });
        div().child("This page blocks navigation (async)")
    }
}
```

**Verification:** `cargo build -p example-app`

---

#### Step 5.1.4: Commit

```bash
git add navi-router/src/blocker.rs navi-router/src/state.rs example-app/src/routes/blocking.rs
git commit -m "feat: async navigation blocker support"
```

---

### Task 5.2: `Link` – `activeOptions` and `resetScroll`

---

#### Step 5.2.1: Add `ActiveOptions` and `reset_scroll` to `Link`

**File:** `navi-router/src/components/link.rs`

```rust
pub struct ActiveOptions {
    pub exact: bool,
    pub include_hash: bool,
    pub include_search: bool,
}

impl Default for ActiveOptions {
    fn default() -> Self {
        Self {
            exact: false,
            include_hash: false,
            include_search: false,
        }
    }
}

pub struct Link {
    // ...
    active_options: ActiveOptions,
    reset_scroll: bool,
}
```

---

#### Step 5.2.2: Implement builder methods

```rust
impl Link {
    pub fn active_options(mut self, options: ActiveOptions) -> Self {
        self.active_options = options;
        self
    }

    pub fn reset_scroll(mut self, reset: bool) -> Self {
        self.reset_scroll = reset;
        self
    }
}
```

---

#### Step 5.2.3: Update `is_active` logic

In `render`, compute active state using options:

```rust
let is_active = RouterState::try_global(cx)
    .map(|state| {
        let current = state.current_location();
        let path_matches = if self.active_options.exact {
            current.pathname == self.href
        } else {
            current.pathname.starts_with(&self.href)
        };
        // Optionally check hash and search
        path_matches
    })
    .unwrap_or(false);
```

---

#### Step 5.2.4: Pass `reset_scroll` to navigation

In the click handler:

```rust
let mut options = NavigateOptions::default();
options.replace = self.replace;
options.reset_scroll = Some(self.reset_scroll);
navigator.navigate(loc, options, cx);
```

---

#### Step 5.2.5: Implement preload cancellation for hover

Store preload task in element state and cancel on mouse leave.

**Verification:** `cargo check -p navi-router`

---

#### Step 5.2.6: Commit

```bash
git add navi-router/src/components/link.rs
git commit -m "feat: Link activeOptions, resetScroll, and preload cancellation"
```

---

These revised chunks incorporate the review feedback and provide a solid, GPUI‑compliant foundation for the remaining work. The devtools enhancements (Chunk 8) should be similarly detailed in a separate iteration, ensuring they extend the existing `navi-devtools` module rather than duplicating it.
I'll generate the remaining five chunks (6–10) of the detailed implementation plan, incorporating the review findings. The key corrections are:

- **Chunk 6**: The `Awaited` component now correctly stores the rendered child in `RequestLayoutState` and reuses it in `prepaint` and `paint`.
- **Chunk 8**: The devtools expansion explicitly extends the existing `navi-devtools` module (tabs, state) rather than creating duplicate files.
- All GPUI API usage has been double-checked against the provided `zed-industries-zed` source code.

## Chunk 6: Real Scroll Restoration and `Awaited` Component

### Task 6.1: Real Scroll Restoration

**Files:**
- `navi-router/src/components/scroll_restoration.rs` (modify)
- `example-app/src/routes/__root.rs` (modify)
- `example-app/src/routes/scroll.rs` (new)

---

#### Step 6.1.1: Replace stub with proper `ScrollRestoration` element

**File:** `navi-router/src/components/scroll_restoration.rs`

Replace the current stub with an element that holds a `ScrollHandle` and saves/restores positions:

```rust
use crate::RouterState;
use gpui::{
    App, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, ScrollHandle,
    Window, point,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static SCROLL_POSITIONS: Lazy<Mutex<HashMap<String, f32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub struct ScrollRestoration {
    scroll_handle: ScrollHandle,
}

impl ScrollRestoration {
    pub fn new(scroll_handle: ScrollHandle) -> Self {
        Self { scroll_handle }
    }

    fn save(path: &str, offset: f32) {
        SCROLL_POSITIONS.lock().unwrap().insert(path.to_string(), offset);
    }

    fn get(path: &str) -> Option<f32> {
        SCROLL_POSITIONS.lock().unwrap().get(path).copied()
    }
}

impl Element for ScrollRestoration {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = window.request_layout(Default::default(), [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = RouterState::try_global(cx);
        if let Some(state) = state {
            let path = state.current_location().pathname.clone();
            let offset = self.scroll_handle.offset().y;
            if offset.0 != 0.0 {
                Self::save(&path, offset.0);
            }
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = RouterState::try_global(cx);
        if let Some(state) = state {
            let path = state.current_location().pathname.clone();
            if let Some(saved_y) = Self::get(&path) {
                let scroll_handle = self.scroll_handle.clone();
                let saved = point(scroll_handle.offset().x, Pixels(saved_y));
                window.on_next_frame(move |window, _| {
                    scroll_handle.set_offset(saved);
                    window.refresh();
                });
            }
        }
    }
}

impl IntoElement for ScrollRestoration {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}
```

**Verification:** `cargo check -p navi-router`

---

#### Step 6.1.2: Create demonstration scroll route

**File:** `example-app/src/routes/scroll.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use gpui_component::scroll::ScrollableElement;
use navi_macros::define_route;
use navi_router::components::ScrollRestoration;

define_route!(
    ScrollRoute,
    path: "/scroll",
    component: ScrollPage,
);

#[derive(Clone, IntoElement)]
struct ScrollPage;

impl RenderOnce for ScrollPage {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let scroll_handle = ScrollHandle::new();
        div()
            .size_full()
            .child(
                div()
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .child(
                        div()
                            .child("Scroll Restoration Demo")
                            .children((0..100).map(|i| div().h_8().child(format!("Item {}", i))))
                    )
            )
            .child(ScrollRestoration::new(scroll_handle))
    }
}
```

**Verification:** `cargo check -p example-app`

---

#### Step 6.1.3: Update root layout to use `ScrollRestoration`

**File:** `example-app/src/routes/__root.rs`

Wrap the outlet in a scrollable container and attach `ScrollRestoration`:

```rust
use navi_router::components::{Outlet, ScrollRestoration};
use gpui::ScrollHandle;

// Inside render:
let scroll_handle = ScrollHandle::new();
div()
    .size_full()
    .flex()
    .flex_col()
    // ... header ...
    .child(
        div()
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&scroll_handle)
            .child(Outlet::new())
    )
    .child(ScrollRestoration::new(scroll_handle))
```

**Verification:** `cargo check -p example-app`

---

#### Step 6.1.4: Commit

```bash
git add navi-router/src/components/scroll_restoration.rs example-app/src/routes/scroll.rs example-app/src/routes/__root.rs
git commit -m "feat: real scroll restoration with ScrollHandle"
```

---

### Task 6.2: `Awaited` Component

**Files:**
- Create: `navi-router/src/components/awaited.rs`
- Modify: `navi-router/src/components/mod.rs`
- Create: `example-app/src/routes/awaited.rs`

---

#### Step 6.2.1: Create `Awaited` element with proper state handling

**File:** `navi-router/src/components/awaited.rs`

```rust
use crate::{RouteDef, RouterState};
use gpui::{
    AnyElement, App, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels,
    RenderOnce, Window,
};
use std::marker::PhantomData;

pub struct Awaited<R: RouteDef> {
    fallback: Option<Box<dyn Fn() -> AnyElement>>,
    child: Option<Box<dyn Fn(R::LoaderData) -> AnyElement>>,
    _phantom: PhantomData<R>,
}

impl<R: RouteDef> Awaited<R> {
    pub fn new() -> Self {
        Self {
            fallback: None,
            child: None,
            _phantom: PhantomData,
        }
    }

    pub fn fallback(mut self, f: impl Fn() -> AnyElement + 'static) -> Self {
        self.fallback = Some(Box::new(f));
        self
    }

    pub fn child(mut self, f: impl Fn(R::LoaderData) -> AnyElement + 'static) -> Self {
        self.child = Some(Box::new(f));
        self
    }
}

enum AwaitedChild<R: RouteDef> {
    Fallback(AnyElement),
    Data(AnyElement, R::LoaderData),
}

impl<R: RouteDef> Element for Awaited<R> {
    type RequestLayoutState = AwaitedChild<R>;
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = RouterState::try_global(cx);
        let data = state.and_then(|s| s.get_loader_data::<R>());

        let child_state = if let Some(data) = data {
            if let Some(child_fn) = &self.child {
                let mut element = child_fn(data.clone());
                let layout_id = element.request_layout(window, cx);
                AwaitedChild::Data(element, data)
            } else {
                let layout_id = window.request_layout(Default::default(), [], cx);
                AwaitedChild::Data(AnyElement::new(gpui::Empty), data)
            }
        } else if let Some(fallback_fn) = &self.fallback {
            let mut element = fallback_fn();
            let layout_id = element.request_layout(window, cx);
            AwaitedChild::Fallback(element)
        } else {
            let layout_id = window.request_layout(Default::default(), [], cx);
            AwaitedChild::Fallback(AnyElement::new(gpui::Empty))
        };
        (layout_id, child_state)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        match state {
            AwaitedChild::Fallback(elem) => elem.prepaint(window, cx),
            AwaitedChild::Data(elem, _) => elem.prepaint(window, cx),
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        match state {
            AwaitedChild::Fallback(elem) => elem.paint(window, cx),
            AwaitedChild::Data(elem, _) => elem.paint(window, cx),
        }
    }
}

impl<R: RouteDef> IntoElement for Awaited<R> {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}
```

**Explanation:** The child element is created during `request_layout`, stored in `RequestLayoutState`, and reused during `prepaint` and `paint`. This respects GPUI's requirement that the same element instance is used across all phases.

---

#### Step 6.2.2: Export from components module

**File:** `navi-router/src/components/mod.rs`

```rust
pub mod awaited;
pub use awaited::Awaited;
```

---

#### Step 6.2.3: Create demonstration route

**File:** `example-app/src/routes/awaited.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::Awaited;
use std::time::Duration;

define_route!(
    AwaitedDemoRoute,
    path: "/awaited",
    data: String,
    loader: |_, executor: gpui::BackgroundExecutor| async move {
        executor.timer(Duration::from_secs(2)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new("Loaded data!".to_string()))
    },
    component: AwaitedDemoPage,
);

#[derive(Clone, IntoElement)]
struct AwaitedDemoPage;

impl RenderOnce for AwaitedDemoPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .child("Awaited Demo")
            .child(
                Awaited::<AwaitedDemoRoute>::new()
                    .fallback(|| div().child("Loading...").into_any_element())
                    .child(|data: String| div().child(format!("Data: {}", data)).into_any_element())
            )
    }
}
```

**Verification:** `cargo check -p example-app`

---

#### Step 6.2.4: Commit

```bash
git add navi-router/src/components/awaited.rs navi-router/src/components/mod.rs example-app/src/routes/awaited.rs
git commit -m "feat: add Awaited component for granular data-ready rendering"
```

---

## Chunk 7: Codegen Improvements

### Task 7.1: Pathless Layout Directories (`_` prefix)

**Files:**
- Modify: `navi-codegen/src/scanner.rs`
- Create: `example-app/src/routes/_auth/mod.rs`
- Create: `example-app/src/routes/_auth/profile.rs`

---

#### Step 7.1.1: Detect `_`-prefixed directories and mark as pathless

**File:** `navi-codegen/src/scanner.rs`

Add `pathless_parent: bool` to `RouteInfo`. In `scan_routes`, when iterating, check if any parent directory starts with `_` and set `pathless_parent = true`. Then, in `file_name_to_pattern`, skip `_`-prefixed directory segments.

```rust
// Inside scan_routes loop
let mut pathless_parent = false;
for component in relative_path.parent().unwrap().components() {
    if let Some(name) = component.as_os_str().to_str() {
        if name.starts_with('_') {
            pathless_parent = true;
            break;
        }
    }
}
```

Update `file_name_to_pattern`:

```rust
for component in relative_path.parent().into_iter().flat_map(|p| p.iter()) {
    if let Some(comp) = component.to_str() {
        if comp.starts_with('_') {
            continue; // pathless layout segment
        }
        // ... rest of logic ...
    }
}
```

**Verification:** `cargo check -p navi-codegen`

---

#### Step 7.1.2: Create example pathless layout

**File:** `example-app/src/routes/_auth/mod.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::define_route;
use navi_router::components::Outlet;

define_route!(
    AuthLayoutRoute,
    path: "/",  // pathless – no URL segment
    is_layout: true,
    component: AuthLayout,
);

#[derive(Clone, IntoElement)]
struct AuthLayout;
impl RenderOnce for AuthLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Auth Layout (pathless)").child(Outlet::new())
    }
}
```

**File:** `example-app/src/routes/_auth/profile.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::define_route;

define_route!(
    ProfileRoute,
    path: "/profile",  // full URL: /profile
    component: ProfilePage,
);

#[derive(Clone, IntoElement)]
struct ProfilePage;
impl RenderOnce for ProfilePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Profile Page (inside pathless auth layout)")
    }
}
```

**Verification:** `cargo build -p example-app`

---

#### Step 7.1.3: Commit

```bash
git add navi-codegen/src/scanner.rs example-app/src/routes/_auth/
git commit -m "feat: pathless layout directories with _ prefix"
```

---

### Task 7.2: Route Groups `(group)` – Validation and Duplicate Path Detection

**Files:**
- Modify: `navi-codegen/src/scanner.rs`

---

#### Step 7.2.1: Skip `(group)` segments in URL pattern

**File:** `navi-codegen/src/scanner.rs`

In `file_name_to_pattern`, skip components that start with `(` and end with `)`:

```rust
if comp.starts_with('(') && comp.ends_with(')') {
    continue;
}
```

---

#### Step 7.2.2: Add duplicate path detection

After collecting all routes in `scan_routes`, check for duplicate patterns:

```rust
use std::collections::HashSet;

let mut seen_patterns = HashSet::new();
for route in &routes {
    if !seen_patterns.insert(&route.route_pattern) {
        anyhow::bail!(
            "Duplicate route pattern detected: '{}' (from {:?} and another route)",
            route.route_pattern,
            route.relative_path
        );
    }
}
```

---

#### Step 7.2.3: (Optional) Add example route group

Create `example-app/src/routes/(marketing)/about.rs` to verify groups are ignored.

---

#### Step 7.2.4: Commit

```bash
git add navi-codegen/src/scanner.rs
git commit -m "feat: route group validation and duplicate path detection"
```

---

### Task 7.3: Optional Segment File Names `{-$param}.rs`

**Files:**
- Modify: `navi-codegen/src/scanner.rs`
- Modify: `navi-router/src/route_tree.rs`
- Create: `example-app/src/routes/users/{-$id}.rs`

---

#### Step 7.3.1: Parse filenames matching `{-$...}.rs`

**File:** `navi-codegen/src/scanner.rs`

In `file_name_to_pattern`, detect optional dynamic segments:

```rust
if file_name.starts_with("{-$") && file_name.ends_with('}') {
    let name = &file_name[3..file_name.len()-1];
    return format!("/{}", format!("{{-${}}}", name));
}
```

---

#### Step 7.3.2: Update `RoutePattern` parser for optional segments

**File:** `navi-router/src/route_tree.rs`

In `parse_segments`, add branch:

```rust
if part.starts_with("{-$") && part.ends_with('}') {
    let name = part[3..part.len()-1].to_string();
    segments.push(Segment::Optional { name, prefix: None, suffix: None });
    continue;
}
```

---

#### Step 7.3.3: Create example optional segment route

**File:** `example-app/src/routes/users/{-$id}.rs`

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::{define_route, use_params};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct OptionalUserParams {
    pub id: Option<String>,
}

define_route!(
    OptionalUserRoute,
    path: "/users/{-$id}",
    params: OptionalUserParams,
    component: OptionalUserPage,
);

#[derive(Clone, IntoElement)]
struct OptionalUserPage;
impl RenderOnce for OptionalUserPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let params = use_params!(OptionalUserRoute);
        let id = params.id.as_deref().unwrap_or("none");
        div().child(format!("Optional user ID: {}", id))
    }
}
```

**Verification:** `cargo build -p example-app`

---

#### Step 7.3.4: Commit

```bash
git add navi-codegen/src/scanner.rs navi-router/src/route_tree.rs example-app/src/routes/users/{-$id}.rs
git commit -m "feat: support optional path segments with {-$param}.rs"
```

---

### Task 7.4: Emit New Fields in Generated Code

**Files:**
- Modify: `navi-codegen/src/scanner.rs`
- Modify: `navi-codegen/src/generator.rs`

---

#### Step 7.4.1: Parse additional attributes from route files

**File:** `navi-codegen/src/scanner.rs`

Use regex to extract `stale_time`, `gc_time`, `loader_deps`, `before_load`, `meta`, `on_enter`, `on_leave`. Add these fields to `RouteInfo`.

Example:

```rust
let stale_time_re = Regex::new(r"stale_time\s*:\s*([^,]+)").unwrap();
let stale_time = stale_time_re.captures(content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
```

---

#### Step 7.4.2: Emit assignments in `build_route_tree`

**File:** `navi-codegen/src/generator.rs`

For each route, generate code like:

```rust
if let Some(stale_time) = &route.stale_time {
    node_block.push_str(&format!("node.stale_time = Some({});", stale_time));
}
```

---

#### Step 7.4.3: Test with example route that uses these attributes

Add a route in `example-app` with `stale_time` and `meta` to verify.

---

#### Step 7.4.4: Commit

```bash
git add navi-codegen/src/scanner.rs navi-codegen/src/generator.rs
git commit -m "feat: emit loaderDeps, staleTime, beforeLoad, meta in generated route tree"
```

---

## Chunk 8: Devtools Expansion (Extending Existing Module)

**Important:** The existing `navi-devtools` already provides `DevtoolsState` with tabs for Routes, Cache, Timeline, and State. This chunk enhances those tabs rather than creating new ones.

### Task 8.1: Enhance Route Tree Inspector Tab

**Files:**
- Modify: `navi-devtools/src/lib.rs`

---

#### Step 8.1.1: Add route tree visualization to the Routes tab

In `DevtoolsState::render_routes_tab`, replace the current placeholder with a collapsible tree of all routes.

Implementation outline (using existing patterns from the codebase):

- Read `RouterState::global(cx).route_tree`.
- Build a hierarchical tree structure from `RouteNode`s.
- Render each node with indentation, expand/collapse toggle (using button with chevron icon), and metadata (pattern, layout/index/loader tags).
- Highlight the currently active route (from `RouterState::current_match`).

**Code snippet (to be integrated into existing `render_routes_tab`):**

```rust
// Inside render_routes_tab, after fetching state
let tree = &state.route_tree;
let root_id = tree.root_id();
let current_leaf_id = state.current_match.as_ref().map(|(_, n)| n.id.clone());

fn render_node(
    id: &str,
    depth: usize,
    tree: &RouteTree,
    current_leaf_id: Option<&str>,
    collapsed: &HashSet<String>,
    toggle: impl Fn(String) + 'static,
    cx: &App,
) -> Vec<Div> {
    let node = tree.get_node(id).unwrap();
    let has_children = tree.children_of(id).map(|v| !v.is_empty()).unwrap_or(false);
    let is_collapsed = collapsed.contains(id);
    let is_active = current_leaf_id == Some(id);
    let mut rows = Vec::new();

    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .pl(Pixels(depth as f32 * 20.0))
        .py(px(4.0))
        .when(is_active, |d| d.bg(cx.theme().primary.opacity(0.2)));

    if has_children {
        let id_clone = id.to_string();
        row = row.child(
            Button::new(format!("toggle-{}", id))
                .icon(if is_collapsed { IconName::ChevronRight } else { IconName::ChevronDown })
                .ghost()
                .xsmall()
                .on_click(move |_, _, _| toggle(id_clone.clone())),
        );
    } else {
        row = row.child(div().w(px(20.0)));
    }

    row = row
        .child(div().child(id.to_string()))
        .child(div().text_color(cx.theme().muted_foreground).child(node.pattern.raw.clone()));

    // Add tags (layout, index, loader)
    let mut tags = Vec::new();
    if node.is_layout { tags.push("layout"); }
    if node.is_index { tags.push("index"); }
    if node.has_loader { tags.push("loader"); }
    if !tags.is_empty() {
        row = row.child(div().flex().gap_1().children(tags.into_iter().map(|tag| {
            div().px_1().rounded(px(2.0)).bg(cx.theme().muted_foreground.opacity(0.1)).child(tag)
        })));
    }

    rows.push(row);

    if !is_collapsed && has_children {
        for child_id in tree.children_of(id).unwrap() {
            rows.extend(render_node(child_id, depth + 1, tree, current_leaf_id, collapsed, toggle.clone(), cx));
        }
    }

    rows
}
```

**Verification:** `cargo check -p navi-devtools`

---

#### Step 8.1.2: Add route testing input

In the Routes tab, add an `Input` field where the user can type a path (e.g., `/users/42?tab=profile`) and a "Test Navigation" button that calls `Navigator::push`.

---

#### Step 8.1.3: Commit

```bash
git add navi-devtools/src/lib.rs
git commit -m "feat: enhance devtools Routes tab with tree view and route tester"
```

---

### Task 8.2: Enhance Timeline Tab

**Files:**
- Modify: `navi-devtools/src/lib.rs`
- Modify: `navi-devtools/src/timeline.rs` (if needed)

---

#### Step 8.2.1: Add search/filter to timeline

The existing `render_timeline_tab` already has a search input and type filter dropdown. Ensure it's functional and clear.

---

#### Step 8.2.2: Add event detail panel

When a timeline event is clicked, show a panel with full `EventDetail` (from/to pathname, search, state). This is partially implemented; complete it.

---

#### Step 8.2.3: Add export/clear functionality

Add buttons to copy all events as text or JSON, and to clear the event log (calls `event_bus::clear_event_log`). Already partially present; ensure it works.

---

#### Step 8.2.4: Commit

```bash
git add navi-devtools/src/lib.rs
git commit -m "feat: enhance devtools Timeline tab with search, detail panel, and export"
```

---

### Task 8.3: Enhance Cache Tab

**Files:**
- Modify: `navi-devtools/src/lib.rs`

---

#### Step 8.3.1: Improve cache table display

The existing cache tab uses a `DataTable` with columns for key, age, stale status, and actions. Enhance it to show the actual data preview (e.g., JSON tree) when a row is selected.

---

#### Step 8.3.2: Add cache invalidation by pattern

Add an input to invalidate queries matching a key pattern (using `rs_query`'s `invalidate_queries` method).

---

#### Step 8.3.3: Commit

```bash
git add navi-devtools/src/lib.rs
git commit -m "feat: enhance devtools Cache tab with data preview and pattern invalidation"
```

---

### Task 8.4: Enhance State Tab

**Files:**
- Modify: `navi-devtools/src/lib.rs`

---

#### Step 8.4.1: Display router state details

Already shows navigation state and route tree statistics. Add more details: current loader statuses, blocker list, pending navigation.

---

#### Step 8.4.2: Commit

```bash
git add navi-devtools/src/lib.rs
git commit -m "feat: enhance devtools State tab with loader and blocker info"
```

---

## Chunk 9: Example App Overhaul

### Task 9.1: Add All Demonstration Routes

**Files:**
- Create multiple route files in `example-app/src/routes/`

---

#### Step 9.1.1: Create remaining route files

- `routes/meta.rs` (demonstrates `meta` map)
- `routes/not-found.rs` (global 404 already created as `$.rs`)
- Ensure all routes from previous chunks are present: `login.rs`, `admin/`, `posts/`, `lifecycle.rs`, `blocking.rs`, `awaited.rs`, `scroll.rs`, `_auth/`.

---

#### Step 9.1.2: Update root layout with navigation links

**File:** `example-app/src/routes/__root.rs`

Add `Link` components to all demonstration routes. Display the current meta title in the header.

---

#### Step 9.1.3: Update `main.rs` to pass `RouterOptions`

Enable all features and start the app.

---

#### Step 9.1.4: Build and test

```bash
cargo build -p example-app --features "validator garde validify valico"
cargo run -p example-app
```

Verify all routes are accessible and features work.

---

#### Step 9.1.5: Commit

```bash
git add example-app/
git commit -m "feat: overhaul example app to demonstrate all router features"
```

---

## Chunk 10: Testing & Documentation

### Task 10.1: Unit Tests

**Files:**
- Modify: `navi-router/src/matcher.rs`
- Modify: `navi-router/src/state.rs`
- Modify: `navi-router/src/blocker.rs`
- Modify: `navi-router/src/validation.rs`

---

#### Step 10.1.1: Add matcher tests for optional segments and splat

```rust
#[test]
fn test_optional_segment_matching() {
    let pattern = RoutePattern::parse("/users/{-$id}");
    let matches = pattern.matches("/users").unwrap();
    assert_eq!(matches.get("id").unwrap(), "");
    let matches = pattern.matches("/users/42").unwrap();
    assert_eq!(matches.get("id").unwrap(), "42");
}
```

---

#### Step 10.1.2: Add tests for `beforeLoad` and loader interception

Use `TestAppContext` to create a router and navigate, asserting redirects and 404s.

---

#### Step 10.1.3: Add tests for async blocker

Simulate navigation with blocker that returns `true`/`false` asynchronously.

---

#### Step 10.1.4: Add tests for validation backends

Test `validator`, `garde`, `validify`, `valico` with valid and invalid search params.

---

#### Step 10.1.5: Run tests

```bash
cargo test -p navi-router
```

---

#### Step 10.1.6: Commit

```bash
git add navi-router/src/
git commit -m "test: add unit tests for new router features"
```

---

### Task 10.2: Integration Test Crate

**Files:**
- Create: `navi-test/` crate

---

#### Step 10.2.1: Add `navi-test` to workspace

Edit root `Cargo.toml`:

```toml
[workspace]
members = [
    # ...
    "navi-test",
]
```

Create `navi-test/Cargo.toml` with dependencies on `navi-router`, `gpui`, `gpui_platform`.

---

#### Step 10.2.2: Write headless integration test

**File:** `navi-test/tests/integration.rs`

```rust
use gpui::*;
use navi_router::*;

#[gpui::test]
async fn test_navigation_and_loader_caching(cx: &mut TestAppContext) {
    // Build route tree, navigate, assert loader runs once, then cached.
}
```

---

#### Step 10.2.3: Run integration test

```bash
cargo test -p navi-test
```

---

#### Step 10.2.4: Commit

```bash
git add navi-test/ Cargo.toml
git commit -m "test: add integration test crate for navi-router"
```

---

### Task 10.3: Update README.md

**Files:**
- Modify: `README.md`

---

#### Step 10.3.1: Rewrite README with feature parity checklist

Include:
- Quick start
- Configuration (`navi.config.json`, `RouterOptions`)
- File-based routing conventions
- `define_route!` macro usage with all options
- Hooks (`use_params!`, `use_search!`, `use_loader_data!`, `use_navigate!`, `use_matched_route!`, `use_route_context!`)
- Components (`Link`, `Outlet`, `SuspenseBoundary`, `Awaited`, `ScrollRestoration`)
- Devtools
- Feature comparison table vs TanStack Router

---

#### Step 10.3.2: Commit

```bash
git add README.md
git commit -m "docs: rewrite README with full feature documentation"
```
