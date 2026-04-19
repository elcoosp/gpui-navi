# gpui-navi Enhancement Plan — TanStack Router Feature Parity

> **Goal:** Bring `gpui-navi` to full feature parity with TanStack Router and update the `example-app` to demonstrate every capability.

---

## Gap Analysis: What Exists vs. What Is Missing

| TanStack Router Feature | Current State | Gap |
|---|---|---|
| File-based routing + codegen | ✅ Full | — |
| Dynamic segments (`$id`) | ✅ Full | — |
| Splat / catch-all (`$`) | ✅ Full | — |
| Layout routes + nested outlets | ✅ Full | — |
| Index routes | ✅ Full | — |
| Loaders (async data) | ✅ Partial | No error boundaries; loader cancel/abort missing |
| Search params (typed) | ✅ Partial | No `defaultSearch`; no `loaderDeps`; no search middleware applied at navigate time |
| Navigation guards / blockers | ✅ Partial | Blocker cannot return a custom message; no async blocker |
| `Link` component | ✅ Partial | No `activeOptions`, no `resetScroll`, no `preload` on hover actually cancels |
| History back/forward/go | ✅ Full | — |
| Navigator (programmatic nav) | ✅ Partial | `preload` exists but no abort; no `invalidate` |
| Scroll restoration | ⚠️ Stub | No actual GPUI scroll position read/write |
| Suspense / `SuspenseBoundary` | ⚠️ Stub | Fallback renders but no `pendingMs` / `pendingMinMs` timer |
| `NotFound` / 404 route | ⚠️ Stub | `not_found()` returns data but no dedicated 404 component rendering |
| Redirects from loaders | ⚠️ Stub | `Redirect` struct exists but router never catches it from a loader result |
| `beforeLoad` hooks | ❌ Missing | No per-route before-load hook |
| `onEnter` / `onLeave` hooks | ❌ Missing | No lifecycle callbacks |
| Route context (`routeContext`) | ❌ Missing | Per-route typed context injection |
| `loaderDeps` | ❌ Missing | Loaders don't re-run when tracked search params change |
| `staleTime` / `gcTime` per route | ❌ Missing | No per-route cache tuning on top of rs-query |
| `defaultPendingMs` / `defaultPendingMinMs` | ❌ Missing | Router-level defaults only in struct |
| `notFoundMode` (`root` vs `fuzzy`) | ❌ Missing | |
| `Awaited` component | ❌ Missing | Granular data-ready rendering inside a route |
| Parallel / waterfall loader control | ❌ Missing | All loaders run independently; no explicit parallel grouping |
| `route.options.meta` (SEO / window title) | ❌ Missing | No per-route metadata |
| `useMatchedRoute` / `useRouteContext` hooks/macros | ❌ Missing | |
| `use_navigate` macro | ❌ Missing | `use_search` exists; `use_navigate` does not |
| Optional path segments `{-$param}` | ✅ Struct exists | Codegen never emits them; no example |
| Pathless / layout-only routes `_layout` | ⚠️ Partial | `is_layout` inference by underscore prefix exists but codegen misses nested pathless dirs |
| `navi.config.json` — `routeGroups` `(group)` | ⚠️ Partial | `(group)` dirs are skipped but not validated or documented |
| Devtools — route inspector | ⚠️ Partial | Query devtools exist; no route tree / match inspector tab |
| Devtools — navigation timeline | ✅ Partial | Events logged; UI only shows query state |

---

## Phase 1 — Core Runtime Completeness

### 1.1 `beforeLoad` Hook

**File:** `navi-router/src/state.rs` + `navi-router/src/route_tree.rs`

Add an optional async hook to `RouteNode` that runs before the loader and can throw a `Redirect` or `NotFound`.

```rust
// route_tree.rs — add to RouteNode
pub before_load: Option<
    Arc<dyn Fn(BeforeLoadContext) -> BoxFuture<'static, BeforeLoadResult> + Send + Sync>
>,

pub struct BeforeLoadContext {
    pub params: HashMap<String, String>,
    pub search: serde_json::Value,
    pub location: Location,
}

pub enum BeforeLoadResult {
    Ok,
    Redirect(Redirect),
    NotFound(NotFound),
}
```

In `RouterState::navigate`, before spawning the loader task, await `before_load` and act on its result.

Expose in `define_route!` macro:
```rust
define_route!(
    AdminRoute,
    path: "/admin",
    before_load: |ctx| async move {
        if !is_authenticated() { BeforeLoadResult::Redirect(redirect("/login")) }
        else { BeforeLoadResult::Ok }
    },
    component: AdminPage,
);
```

---

### 1.2 Loader `Redirect` + `NotFound` Interception

**File:** `navi-router/src/state.rs` — `run_loader` / `preload_location`

Loaders currently return `Arc<dyn Any>`. Wrap the return in an enum:

```rust
pub enum LoaderOutcome {
    Data(AnyData),
    Redirect(Redirect),
    NotFound(NotFound),
}
```

After the async task resolves, check the variant:
- `Redirect` → call `state.navigate(redirect.to, redirect.options, cx)`
- `NotFound` → set `RouterState::not_found_data` and navigate to the configured 404 route

---

### 1.3 `onEnter` / `onLeave` Lifecycle Hooks

**File:** `navi-router/src/route_tree.rs`

```rust
pub on_enter: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
pub on_leave: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
```

Call `on_leave` for each route in the *previous* match that is not in the *next* match (set difference on route IDs), then call `on_enter` for each newly entered route, after navigation commits.

---

### 1.4 `loaderDeps` — Reactive Search Dependencies

**File:** `navi-router/src/state.rs`, `route_tree.rs`

```rust
pub loader_deps: Option<Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>>,
```

When computing the rs-query cache key, include `loader_deps(location.search)` in the key. This makes loaders re-run when specific search params change without invalidating on every param.

```rust
// Example in define_route!
loader_deps: |search| json!({ "page": search["page"] }),
```

---

### 1.5 Per-Route `staleTime` / `gcTime`

**File:** `navi-router/src/route_tree.rs`

```rust
pub stale_time: Option<Duration>,  // default: 0
pub gc_time: Option<Duration>,     // default: 30min
```

Pass these through to `rs-query`'s query options when spawning a loader task.

---

### 1.6 `defaultPendingMs` / `defaultPendingMinMs` on Router

**File:** `navi-router/src/state.rs` — `RouterState`

```rust
pub default_pending_ms: u64,      // 1000
pub default_pending_min_ms: u64,  // 500
```

`SuspenseBoundary` already has the config struct in `navi-core/src/suspense.rs`; wire them together: `RouterProvider::new` should accept a `RouterOptions` struct, and `SuspenseBoundary` should read these values from `RouterState`.

---

### 1.7 `notFoundMode` + Dedicated 404 Route

**File:** `navi-router/src/state.rs`, `navi-codegen/src/generator.rs`

```rust
pub enum NotFoundMode { Root, Fuzzy }
```

- `Root`: always render the root layout's 404 outlet.
- `Fuzzy`: render the deepest matched layout's 404 outlet.

In codegen, treat a file named `$.rs` at the root of `routes/` (with no prefix path) as the global 404 component. File `routes/users/$.rs` is the users-scoped 404.

---

### 1.8 Route Context (`routeContext`)

**File:** `navi-router/src/route_tree.rs`, `navi-macros/src/route.rs`

```rust
pub context_fn: Option<Arc<dyn Fn(RouteContextArgs) -> serde_json::Value + Send + Sync>>,
```

`RouteContextArgs` contains parent context, params, and loader data. The result is stored alongside loader data in the rs-query cache. Expose via a new macro `use_route_context!(RouteType)`.

---

## Phase 2 — `navi-macros` Completeness

### 2.1 `use_navigate!` Macro

**File:** `navi-macros/src/hooks.rs`

```rust
// Returns a Navigator bound to the current window handle
let nav = use_navigate!();
nav.push("/settings", cx);
nav.replace("/login", cx);
```

Implementation: read `RouterState::try_global(cx).map(|s| Navigator::new(s.window_handle))`.

---

### 2.2 `use_matched_route!` Macro

Returns the currently active `RouteNode` for the component's own route, giving access to params, search, context, and meta.

```rust
let route = use_matched_route!(UsersParamIdRoute);
let title = route.meta.get("title").cloned().unwrap_or_default();
```

---

### 2.3 `use_route_context!` Macro

Reads per-route context injected by `context_fn` (see §1.8).

---

### 2.4 Per-Route `meta` Map

**File:** `navi-router/src/route_tree.rs`

```rust
pub meta: HashMap<String, serde_json::Value>,
```

Expose in `define_route!`:
```rust
define_route!(
    AboutRoute,
    path: "/about",
    meta: { "title" => "About Us", "description" => "Learn about Navi" },
    component: AboutPage,
);
```

The root layout can read `RouterState::global(cx).current_meta()` to update a window title label.

---

### 2.5 Async Blocker Support

**File:** `navi-router/src/blocker.rs`

Change `Blocker` predicate from `Fn(&Location, &Location) -> bool` to an async version returning `BoxFuture<'static, bool>` so blockers can show a confirmation dialog before resolving. Provide a sync convenience constructor that wraps a closure.

---

## Phase 3 — Component Layer Polish

### 3.1 `Link` — `activeOptions` + `activeClass`

**File:** `navi-router/src/components/link.rs`

```rust
pub struct ActiveOptions {
    pub exact: bool,        // match full path (default: false)
    pub include_hash: bool,
    pub include_search: bool,
}
```

`Link::new(href).active_options(ActiveOptions { exact: true, .. })` adds a visual "active" style when `RouterState::current_location().pathname` matches.

---

### 3.2 `Link` — `resetScroll` + `preload` cancellation

- Add `reset_scroll: bool` to `Link` (default `true`) — passed through to `NavigateOptions`.
- On `PreloadType::Intent`, cancel the in-flight preload if the mouse leaves before it resolves.

---

### 3.3 `Outlet` — Depth-Aware `notFound` Slot

If the `Outlet` finds no registered component for the matched node at its depth, it should check for a `NotFoundComponent` registered at that layout level before falling back to the text warning.

---

### 3.4 Real Scroll Restoration

**File:** `navi-router/src/components/scroll_restoration.rs`

GPUI exposes scroll state via `ScrollHandle`. `ScrollRestoration` needs to:

1. Accept a `ScrollHandle` reference.
2. On route leave: read `scroll_handle.offset()` and save to `SCROLL_POSITIONS`.
3. On route enter: if a saved position exists, call `scroll_handle.set_offset(y)` after the next render tick using `cx.spawn`.

```rust
ScrollRestoration::new(scroll_handle.clone())
```

---

### 3.5 `Awaited` Component

Mirrors TanStack's `<Awaited>` — renders a fallback until a specific loader's data is ready, independently of the global `SuspenseBoundary`.

```rust
Awaited::new::<UsersParamIdRoute>()
    .fallback(|| div().child("Loading user…").into_any_element())
    .child(UserCard)
```

---

## Phase 4 — Codegen (`navi-codegen`) Improvements

### 4.1 Pathless Layout Directories

Directories prefixed with `_` (e.g., `routes/_auth/`) should be treated as pathless layout segments: they provide a layout route with no URL segment of their own. Codegen must:

- Detect `_`-prefixed directories.
- Assign children to the layout `mod.rs` / `_auth.rs` inside the dir.
- Not add a URL segment for the dir name.

### 4.2 Route Groups `(group)` — Validation + Docs

Codegen already skips `(group)` dir segments in the URL but does not validate that no two groups produce conflicting paths. Add a compile-time duplicate-path check in `scan_routes` that returns an error listing conflicts.

### 4.3 Emit `loaderDeps`, `staleTime`, `beforeLoad`, `meta` in Generated Tree

The scanner should parse additional keys from `define_route!` and emit them in `build_route_tree()`:

```rust
node.stale_time = Some(Duration::from_secs(60));
node.meta.insert("title".into(), json!("Users"));
```

### 4.4 Optional Segment File Names `{-$param}.rs`

Map file name `{-$id}.rs` → `Segment::Optional { name: "id", .. }`. Currently the scanner only handles plain `$name` and `$` (splat).

---

## Phase 5 — Devtools Expansion

### 5.1 Route Tree Inspector Tab

**File:** `navi-devtools/src/lib.rs` — add `DevtoolsTab::Routes`

Render a collapsible tree of all registered `RouteNode`s showing:
- Pattern, ID, `is_layout`, `is_index`
- Current match highlight (bold/colored)
- Extracted params for the active match
- Loader status badge (Idle / Loading / Ready / Error)

### 5.2 Navigation Timeline UI

**File:** `navi-devtools/src/timeline.rs`

The `DevtoolsEvent` variants are already defined. Connect them to a scrollable timeline panel in the devtools UI showing a chronological list of navigation events, loader events, and blocker events with timestamps.

### 5.3 Search Params Inspector

Show parsed and validated search params for the active route in the devtools panel, using the same JSON tree widget already used for query data.

---

## Phase 6 — Example App Overhaul

The example-app should have one dedicated route per feature. Add the following routes:

### New Routes

| File | Path | Demonstrates |
|---|---|---|
| `routes/login.rs` | `/login` | Redirect target for auth guard |
| `routes/admin/mod.rs` | `/admin` (layout) | `beforeLoad` auth guard + route context |
| `routes/admin/index.rs` | `/admin` (index) | reads `use_route_context!` |
| `routes/admin/dashboard.rs` | `/admin/dashboard` | `meta` title + `staleTime` |
| `routes/posts/mod.rs` | `/posts` (layout) | `loaderDeps` on `?page=` search param |
| `routes/posts/index.rs` | `/posts` | paginated list, `use_search!` + `use_loader_data!` |
| `routes/posts/$id.rs` | `/posts/$id` | parallel with `/posts` layout loader |
| `routes/lifecycle.rs` | `/lifecycle` | `onEnter` / `onLeave` logged to screen |
| `routes/blocking.rs` | `/blocking` | async blocker with inline confirm dialog |
| `routes/$.rs` | `/*` (global 404) | `notFoundMode` = Root fallback |
| `routes/users/$.rs` | `/users/*` | scoped 404 within users layout |
| `routes/_auth/mod.rs` | (pathless layout) | shared auth check without extra URL segment |
| `routes/awaited.rs` | `/awaited` | `Awaited` component with per-field loading |
| `routes/scroll.rs` | `/scroll` | real `ScrollRestoration` with `ScrollHandle` |
| `routes/meta.rs` | `/meta` | per-route `meta` consumed by root layout title bar |

### Root Layout Updates (`routes/__root.rs`)

- Add a dynamic window title label that reads `RouterState::global(cx).current_meta()["title"]`.
- Add back/forward navigation buttons using `Navigator::can_go_back` / `can_go_forward`.
- Add a "preload on hover" link to `/posts` demonstrating `PreloadType::Intent`.
- Include `ScrollRestoration` with the main scroll handle.

### `main.rs` Updates

- Pass `RouterOptions { default_pending_ms: 500, default_pending_min_ms: 200, not_found_mode: NotFoundMode::Fuzzy }` to `RouterProvider`.
- Show devtools with the new Routes + Timeline tabs enabled.

---

## Phase 7 — Testing & Documentation

### 7.1 Unit Tests

Add tests in `navi-router/src/` for:

- `matcher.rs` — optional segments, splat, ranked ordering, conflicting patterns
- `state.rs` — `beforeLoad` redirect interception, loader `NotFound` interception, `loaderDeps` key invalidation
- `blocker.rs` — sync and async blocker resolution
- `validation.rs` — all four validation backends with valid + invalid inputs

### 7.2 Integration Test Crate

Add `navi-test/` to the workspace with a headless GPUI test harness that:

- Builds a minimal route tree
- Calls `RouterState::navigate` and asserts the resulting location
- Asserts loader data is cached / invalidated correctly

### 7.3 `README.md` Rewrite

Document every feature with a minimal code snippet, organized by the same phases above. Include a feature-parity checklist table comparing gpui-navi with TanStack Router.

---

## Implementation Order (Recommended)

```
Phase 1.1  beforeLoad
Phase 1.2  Loader redirect/notFound interception
Phase 3.1  Link activeOptions
Phase 1.7  notFoundMode + 404 route rendering
Phase 1.4  loaderDeps
Phase 2.1  use_navigate! macro
Phase 1.3  onEnter/onLeave
Phase 3.4  Real scroll restoration
Phase 1.8  Route context
Phase 3.5  Awaited component
Phase 1.5  staleTime/gcTime per route
Phase 1.6  defaultPendingMs wiring
Phase 1.6  Async blocker
Phase 2.2  use_matched_route! / use_route_context!
Phase 2.4  meta map
Phase 4    Codegen improvements
Phase 5    Devtools expansion
Phase 6    Example app overhaul
Phase 7    Tests + docs
```
The enhancement plan for `gpui-navi` is well‑reasoned and aligns closely with the GPUI framework's architecture. Below is a feature‑by‑feature review, highlighting where GPUI idioms may require specific implementation patterns and where the plan is already on solid ground.

---

## Overall Compatibility Assessment

**Strengths:**
- The plan correctly leverages GPUI's **global state** (`RouterState` as `Global`) and the `rs-query` integration for data fetching.
- It respects the element‑based UI model (`Outlet`, `Link`, `SuspenseBoundary`) and the `Render` / `Element` lifecycle.
- The use of `define_route!` and code generation mirrors patterns seen in Zed’s own action macros.

**Areas Requiring Careful GPUI Integration:**
- **Asynchronous navigation hooks** (`beforeLoad`, async blockers) must be implemented using **spawned tasks** that update router state; navigation cannot be “paused” synchronously.
- **Loader result interception** (redirect / not found) must happen inside the task’s completion handler and trigger a new navigation via `Navigator`.
- **Scroll restoration** requires access to `ScrollHandle`, which is tied to a specific element; the plan's approach is valid.
- **`Awaited` component** needs to refresh the window when loader data arrives—already handled by the existing `RouterState::trigger_loader` logic.

---

## Detailed Review by Phase

### Phase 1 – Core Runtime Completeness

#### 1.1 `beforeLoad` Hook
- **GPUI Compatibility:** ✅ Feasible, but **navigation must be non‑blocking**.
- **Implementation Note:**  
  In `RouterState::navigate`, before triggering the loader, spawn a task that runs the `before_load` closure. Store a `pending_before_load` flag. When the task completes:
  - If `Ok` → proceed to loader.
  - If `Redirect` → call `navigator.push` / `replace`.
  - If `NotFound` → set `not_found_data` and navigate to 404.
  The `before_load` closure should receive an `AsyncApp` or `AsyncWindowContext` to perform async checks (e.g., authentication).

#### 1.2 Loader `Redirect` / `NotFound` Interception
- **GPUI Compatibility:** ✅ Already partially implemented.
- **Current State:** Loaders return `AnyData`. The plan to wrap in `LoaderOutcome` is clean.
- **Action:** After the `rs-query` task resolves, inspect the outcome and dispatch a new navigation via `Navigator`. The window will refresh automatically.

#### 1.3 `onEnter` / `onLeave` Hooks
- **GPUI Compatibility:** ✅ Simple synchronous callbacks. No issues.

#### 1.4 `loaderDeps`
- **GPUI Compatibility:** ✅ Straightforward – just include the extracted JSON in the `rs-query` cache key.

#### 1.5 Per‑Route `staleTime` / `gcTime`
- **GPUI Compatibility:** ✅ `rs-query` already accepts these options. Pass them from `RouteNode` to the query builder.

#### 1.6 `defaultPendingMs` / `defaultPendingMinMs`
- **GPUI Compatibility:** ✅ The `SuspenseBoundary` element can read `RouterState` (via `cx.global()`) during `prepaint`/`paint` and use the configured delays.

#### 1.7 `notFoundMode` + 404 Route
- **GPUI Compatibility:** ✅ The `Outlet` component already traverses the route tree by depth. The logic for “root” vs “fuzzy” 404 can be implemented there.

#### 1.8 Route Context (`routeContext`)
- **GPUI Compatibility:** ✅ Can be stored alongside loader data in `rs-query` or a separate global map.  
- **Macro:** `use_route_context!(RouteType)` can expand to a read from `RouterState::global(cx)`.

---

### Phase 2 – Macros Completeness

#### 2.1 `use_navigate!`
- **GPUI Compatibility:** ✅ The macro can expand to:
  ```rust
  Navigator::new(cx.window_handle())
  ```
  where `cx` is `&mut Window` or `&mut App` (via `WindowHandle`).

#### 2.2 `use_matched_route!`
- **GPUI Compatibility:** ✅ Access `RouterState::current_match` and return the `RouteNode` and params.

#### 2.3 `use_route_context!`
- **GPUI Compatibility:** ✅ Similar to `use_loader_data!`, read from a context store keyed by route ID.

#### 2.4 Per‑Route `meta` Map
- **GPUI Compatibility:** ✅ Store in `RouteNode`. A root layout can read `RouterState::current_meta()` and update a `Label` or window title.  
- **Note:** Setting the native window title requires `Window::set_title()`, which should be called from within a view’s `render` or an effect.

#### 2.5 Async Blocker Support
- **GPUI Compatibility:** ✅ Replace the `bool` return with a future.  
- **Implementation:** In `RouterState::navigate`, if any blocker returns `Pending`, store the navigation as pending and await all blockers concurrently. On resolution, commit or cancel.

---

### Phase 3 – Component Layer Polish

#### 3.1 `Link` – `activeOptions`
- **GPUI Compatibility:** ✅ Compute `is_active` using `RouterState::current_location()` during `render`.

#### 3.2 `Link` – `resetScroll` + Preload Cancellation
- **GPUI Compatibility:** ✅ `reset_scroll` can be passed to `NavigateOptions`.  
- **Preload cancellation:** When the link is hovered, spawn a task and store its `Task` handle; on mouse leave, drop the task to cancel (GPUI’s `Task` cancels when dropped).

#### 3.3 `Outlet` – `notFound` Slot
- **GPUI Compatibility:** ✅ If no component is registered for the matched route ID, fall back to a registered `NotFound` component for that layout depth.

#### 3.4 Real Scroll Restoration
- **GPUI Compatibility:** ✅ GPUI provides `ScrollHandle`.  
- **Implementation:**  
  - On route leave: read `scroll_handle.offset()` and store in a global map.  
  - On route enter: after the next frame (using `cx.spawn` or `Window::on_next_frame`), call `scroll_handle.set_offset()`.

#### 3.5 `Awaited` Component
- **GPUI Compatibility:** ✅ Create an element that reads the loader state for a specific route from `RouterState`.  
- **Refresh:** The loader task already calls `window.refresh()` upon completion, so the `Awaited` component will re‑render automatically.

---

### Phase 4 – Codegen Improvements

All proposed codegen enhancements (pathless layouts, route groups validation, optional segments, emitting new fields) are **independent of GPUI runtime semantics** and can be implemented as described.

---

### Phase 5 – Devtools Expansion

- **Route Tree Inspector:** Can be a GPUI view that reads `RouterState` and renders a collapsible tree.  
- **Timeline UI:** Render a scrollable list of logged events.  
- **Search Params Inspector:** Display the current `location.search` JSON.  
- **Integration:** The existing `navi-devtools` already embeds a view; extending it is straightforward.

---

### Phase 6 – Example App Overhaul

The proposed routes exercise all features in a realistic way. No GPUI‑specific blockers.

---

### Phase 7 – Testing & Documentation

- **Unit Tests:** GPUI’s `#[gpui::test]` with `TestAppContext` works perfectly for testing router logic.  
- **Integration Tests:** A headless test harness can be built using `TestAppContext` and `VisualTestContext`.

---

## Key GPUI‑Specific Implementation Considerations

| Concern | Recommendation |
|---------|----------------|
| **Async navigation hooks must not block** | Use `cx.spawn()` and a state machine (`pending_navigation`). After async work finishes, call `Navigator` methods to proceed. |
| **Accessing window handle in macros** | `use_navigate!` can rely on the fact that `Context<'_>` and `Window` both provide a way to get `AnyWindowHandle`. |
| **Refreshing after loader data arrives** | Already handled by `RouterState::trigger_loader_with_locations`, which calls `window.refresh()`. |
| **Scroll restoration timing** | Use `Window::on_next_frame` or `cx.spawn()` with a short delay to ensure layout has occurred before restoring offset. |
| **Devtools as a separate view** | The existing `DevtoolsState` is an `Entity` that implements `Render`. Adding tabs is a matter of UI composition. |

---

## Conclusion

The plan is **fully compatible with GPUI** and thoughtfully maps TanStack Router concepts to GPUI’s paradigms. The few areas that require adaptation (async hooks, scroll restoration timing) are well within GPUI’s capabilities and are correctly identified in the plan. Proceeding with the proposed implementation order will yield a robust, feature‑complete router for GPUI applications.
I'll write the full implementation plan for bringing `gpui-navi` to feature parity with TanStack Router, following the writing-plans skill guidelines.

I'm using the writing-plans skill to create the implementation plan.

The plan will be structured into multiple chunks, each with detailed, actionable steps including exact file paths, code snippets, commands, and expected outputs. After each chunk, the plan-document-reviewer subagent will verify completeness.

---

```markdown
# Navi Router Enhancement Plan — TanStack Router Feature Parity

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring `gpui-navi` to full feature parity with TanStack Router, including loader redirects, beforeLoad hooks, route context, scroll restoration, and comprehensive devtools.

**Architecture:** Enhance the existing file‑based routing system with new runtime hooks, a more powerful loader interception mechanism, and an expanded devtools panel. Changes are isolated to `navi-router`, `navi-macros`, `navi-codegen`, and `navi-devtools`, with a demonstration‑rich `example-app`.

**Tech Stack:** Rust, GPUI, `rs-query`, `serde`, `syn`/`quote` for macros, `walkdir`/`regex` for codegen.

---
```

## Chunk 1: `beforeLoad` Hooks and Loader Redirect/NotFound Interception

### Task 1.1: Add `before_load` field to `RouteNode`

**Files:**
- Modify: `navi-router/src/route_tree.rs`
- Modify: `navi-router/src/state.rs`
- Modify: `navi-macros/src/route.rs`
- Modify: `example-app/src/routes/admin/mod.rs` (demonstration)

- [ ] **Step 1: Define `BeforeLoadContext` and `BeforeLoadResult` in `route_tree.rs`**

Add after the existing `RouteNode` definition:

```rust
// navi-router/src/route_tree.rs

use crate::location::Location;
use crate::redirect::{NotFound, Redirect};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;

pub struct BeforeLoadContext {
    pub params: HashMap<String, String>,
    pub search: serde_json::Value,
    pub location: Location,
}

pub enum BeforeLoadResult {
    Ok,
    Redirect(Redirect),
    NotFound(NotFound),
}

pub type BeforeLoadFn = Arc<
    dyn Fn(BeforeLoadContext) -> BoxFuture<'static, BeforeLoadResult> + Send + Sync,
>;

pub struct RouteNode {
    // ... existing fields ...
    pub before_load: Option<BeforeLoadFn>,
}
```

- [ ] **Step 2: Update `RouteNode` debug impl to ignore function pointer**

Add `before_load` to `Debug` impl with placeholder.

- [ ] **Step 3: Modify `RouterState::navigate` to execute `before_load` before loader**

In `navi-router/src/state.rs`, within `navigate`, after blockers and before triggering loader:

```rust
// Inside RouterState::navigate, before current_match assignment and loader trigger
let before_load_futures: Vec<_> = self
    .route_tree
    .ancestors(&matched_node.id)
    .iter()
    .chain(std::iter::once(&matched_node))
    .filter_map(|node| node.before_load.as_ref().map(|f| (node.id.clone(), f.clone())))
    .map(|(route_id, before_load)| {
        let ctx = BeforeLoadContext {
            params: params.clone(),
            search: loc.search.clone(),
            location: loc.clone(),
        };
        let fut = before_load(ctx);
        (route_id, fut)
    })
    .collect();

if !before_load_futures.is_empty() {
    let window_handle = self.window_handle;
    cx.spawn(|cx| async move {
        for (route_id, fut) in before_load_futures {
            match fut.await {
                BeforeLoadResult::Ok => continue,
                BeforeLoadResult::Redirect(redirect) => {
                    let nav = Navigator::new(window_handle);
                    cx.update(|cx| {
                        nav.push_location(Location::new(&redirect.to), cx);
                    }).ok();
                    return;
                }
                BeforeLoadResult::NotFound(not_found) => {
                    // Store not found data and navigate to 404
                    cx.update(|cx| {
                        RouterState::update(cx, |state, cx| {
                            state.not_found_data = not_found.data;
                            // Determine 404 route based on not_found_mode
                            let not_found_path = match state.not_found_mode {
                                NotFoundMode::Root => "/404",
                                NotFoundMode::Fuzzy => &loc.pathname, // handled by outlet later
                            };
                            let nav = Navigator::new(state.window_handle);
                            nav.push(not_found_path, cx);
                        });
                    }).ok();
                    return;
                }
            }
        }
        // All beforeLoad hooks passed; proceed with normal navigation
        cx.update(|cx| {
            RouterState::update(cx, |state, cx| {
                state.commit_navigation(loc, options, cx);
            });
        }).ok();
    }).detach();
    return; // Navigation will continue in spawned task
}
```

- [ ] **Step 4: Add `commit_navigation` helper to `RouterState`**

Refactor the existing navigation commit (history push/replace, match update, loader trigger) into a private method `commit_navigation`.

- [ ] **Step 5: Extend `define_route!` macro to accept `before_load`**

In `navi-macros/src/route.rs`, add parsing for `before_load` field:

```rust
// Inside Field matching
"before_load" => {
    if let FieldValue::Expr(expr) = field.value {
        before_load_closure = Some(expr);
    }
}
```

Generate the appropriate code to wrap the user's async closure in `BeforeLoadFn` and store it in the node.

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

- [ ] **Step 6: Update `RouteDef` impl to include `before_load_fn` in `build_node`**

Call `Self::before_load_fn()` and assign to `node.before_load`.

- [ ] **Step 7: Write demonstration route in example-app**

Create `example-app/src/routes/admin/mod.rs`:

```rust
use navi_router::RouteDef;
use gpui::prelude::*;
use navi_macros::define_route;
use navi_router::{BeforeLoadResult, redirect, components::Outlet};

define_route!(
    AdminRoute,
    path: "/admin",
    is_layout: true,
    before_load: |ctx| async move {
        // Simulate auth check
        let is_authenticated = false; // toggle to test
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
        div().child("Admin Area").child(Outlet::new())
    }
}
```

- [ ] **Step 8: Commit**

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/admin/mod.rs
git commit -m "feat: add beforeLoad hook and loader redirect/notFound interception"
```

---

### Task 1.2: Loader Outcome Enum for Redirect/NotFound

**Files:**
- Modify: `navi-router/src/state.rs`
- Modify: `navi-router/src/route_tree.rs`
- Modify: `navi-macros/src/route.rs`

- [ ] **Step 1: Define `LoaderOutcome` enum in `state.rs`**

```rust
pub enum LoaderOutcome<T> {
    Data(T),
    Redirect(Redirect),
    NotFound(NotFound),
}
```

- [ ] **Step 2: Change loader signature to return `LoaderOutcome<Arc<dyn Any + Send + Sync>>`**

Update `LoaderFactory` type alias and the closure signature.

- [ ] **Step 3: In `trigger_loader_with_locations`, handle outcome variants**

After the loader future resolves:

```rust
match outcome {
    LoaderOutcome::Data(data) => {
        client.set_query_data(&key, AnyData(data), options);
        // ... existing success handling ...
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
                // Navigate to 404 route
                let nav = Navigator::new(state.window_handle);
                nav.push("/404", cx);
            });
        }).ok();
    }
}
```

- [ ] **Step 4: Update `define_route!` to wrap user loader in `LoaderOutcome::Data`**

In the generated loader factory, call the user's loader and wrap the result.

- [ ] **Step 5: Add example route that returns `NotFound`**

Create `example-app/src/routes/admin/dashboard.rs` with a loader that conditionally returns `NotFound`.

- [ ] **Step 6: Commit**

```bash
git add navi-router/src/state.rs navi-router/src/route_tree.rs navi-macros/src/route.rs example-app/src/routes/admin/dashboard.rs
git commit -m "feat: support loader returning Redirect or NotFound via LoaderOutcome"
```

---

### Task 1.3: `onEnter` / `onLeave` Lifecycle Hooks

**Files:**
- Modify: `navi-router/src/route_tree.rs`
- Modify: `navi-router/src/state.rs`
- Modify: `navi-macros/src/route.rs`

- [ ] **Step 1: Add fields to `RouteNode`**

```rust
pub on_enter: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
pub on_leave: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
```

- [ ] **Step 2: In `RouterState::commit_navigation`, compute route set differences**

Before updating `current_match`, collect previous route IDs from ancestors + leaf. After computing new match, collect new route IDs. Call `on_leave` for old IDs not in new, and `on_enter` for new IDs not in old.

- [ ] **Step 3: Extend macro to accept `on_enter` and `on_leave`**

```rust
"on_enter" => { on_enter = Some(expr); }
"on_leave" => { on_leave = Some(expr); }
```

- [ ] **Step 4: Add demonstration route in example-app**

`example-app/src/routes/lifecycle.rs` that logs to console.

- [ ] **Step 5: Commit**

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/lifecycle.rs
git commit -m "feat: add onEnter/onLeave route lifecycle hooks"
```

---

## Chunk 2: `loaderDeps`, `staleTime`/`gcTime`, and Suspense Timing

### Task 2.1: `loaderDeps` – Reactive Search Dependencies

**Files:**
- Modify: `navi-router/src/route_tree.rs`
- Modify: `navi-router/src/state.rs`
- Modify: `navi-macros/src/route.rs`

- [ ] **Step 1: Add `loader_deps` field to `RouteNode`**

```rust
pub loader_deps: Option<Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>>,
```

- [ ] **Step 2: In `trigger_loader_with_locations`, compute deps hash**

If `loader_deps` exists, call it with `location.search` and include the resulting JSON in the `rs-query` cache key.

- [ ] **Step 3: Update macro to parse `loader_deps`**

```rust
"loader_deps" => {
    if let FieldValue::Expr(expr) = field.value {
        loader_deps = Some(expr);
    }
}
```

Generate code to store the closure in `RouteNode`.

- [ ] **Step 4: Add example in `example-app/src/routes/posts/index.rs`**

Paginated list that re-fetches when `?page=` changes.

- [ ] **Step 5: Commit**

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/posts/index.rs
git commit -m "feat: add loaderDeps to make loaders reactive to search params"
```

---

### Task 2.2: Per‑Route `staleTime` and `gcTime`

**Files:**
- Modify: `navi-router/src/route_tree.rs`
- Modify: `navi-macros/src/route.rs`

- [ ] **Step 1: Add fields to `RouteNode`**

```rust
pub stale_time: Option<std::time::Duration>,
pub gc_time: Option<std::time::Duration>,
```

- [ ] **Step 2: Pass to `rs-query` options in `trigger_loader_with_locations`**

Use `node.stale_time.unwrap_or(Duration::ZERO)` and `node.gc_time.unwrap_or(Duration::from_secs(300))`.

- [ ] **Step 3: Extend macro to accept `stale_time` and `gc_time`**

```rust
"stale_time" => { stale_time = Some(expr); }
"gc_time" => { gc_time = Some(expr); }
```

- [ ] **Step 4: Commit**

```bash
git add navi-router/src/route_tree.rs navi-macros/src/route.rs
git commit -m "feat: per-route staleTime and gcTime configuration"
```

---

### Task 2.3: Router‑Level `defaultPendingMs` / `defaultPendingMinMs`

**Files:**
- Modify: `navi-router/src/state.rs`
- Modify: `navi-router/src/components/suspense_boundary.rs`

- [ ] **Step 1: Add fields to `RouterState`**

```rust
pub default_pending_ms: u64,
pub default_pending_min_ms: u64,
```

- [ ] **Step 2: Update `RouterProvider::new` to accept `RouterOptions`**

Create `RouterOptions` struct with these fields and pass to `RouterState::new`.

- [ ] **Step 3: Modify `SuspenseBoundary` to read these values**

In `SuspenseBoundary::render`, access `RouterState::global(cx).default_pending_ms` and implement the timing logic (show fallback only after `pending_ms` has elapsed, and keep it visible for at least `pending_min_ms`).

- [ ] **Step 4: Update `example-app/src/main.rs` to pass options**

- [ ] **Step 5: Commit**

```bash
git add navi-router/src/state.rs navi-router/src/components/suspense_boundary.rs navi-router/src/components/router_provider.rs example-app/src/main.rs
git commit -m "feat: router-level defaultPendingMs and defaultPendingMinMs for suspense"
```

---

## Chunk 3: `notFoundMode`, 404 Routes, and Route Context

### Task 3.1: `notFoundMode` and Dedicated 404 Route Components

**Files:**
- Modify: `navi-router/src/state.rs`
- Modify: `navi-router/src/components/outlet.rs`
- Modify: `navi-codegen/src/generator.rs`

- [ ] **Step 1: Define `NotFoundMode` enum in `state.rs`**

```rust
pub enum NotFoundMode { Root, Fuzzy }
```

- [ ] **Step 2: Add `not_found_mode` to `RouterState` and `RouterOptions`**

- [ ] **Step 3: In `Outlet`, when no component is registered, check for 404 component**

If `not_found_mode == Root`, look for a global `NotFound` component (registered under a special ID like `__not_found__`). If `Fuzzy`, walk up the ancestor chain to find the first layout with a registered `NotFound` component.

- [ ] **Step 4: Update codegen to treat `$.rs` as 404 route**

In `scanner.rs`, when encountering `$.rs`, generate a route with a special `is_not_found` flag. The component should be registered under both its path and the `__not_found__` ID for the appropriate scope.

- [ ] **Step 5: Add example 404 routes in `example-app`**

Create `routes/$.rs` (global) and `routes/users/$.rs` (scoped).

- [ ] **Step 6: Commit**

```bash
git add navi-router/src/state.rs navi-router/src/components/outlet.rs navi-codegen/src/scanner.rs navi-codegen/src/generator.rs example-app/src/routes/$.rs example-app/src/routes/users/$.rs
git commit -m "feat: notFoundMode and dedicated 404 route components"
```

---

### Task 3.2: Route Context (`routeContext`)

**Files:**
- Modify: `navi-router/src/route_tree.rs`
- Modify: `navi-router/src/state.rs`
- Modify: `navi-macros/src/route.rs`
- Create: `navi-macros/src/hooks.rs` (new macro)

- [ ] **Step 1: Add `context_fn` to `RouteNode`**

```rust
pub context_fn: Option<Arc<dyn Fn(RouteContextArgs) -> serde_json::Value + Send + Sync>>,
```

`RouteContextArgs` includes parent context, params, and loader data.

- [ ] **Step 2: Compute and store context alongside loader data**

After loader resolves, call `context_fn` if present and store in `rs-query` under a separate key (e.g., `navi_context:<route_id>`).

- [ ] **Step 3: Implement `use_route_context!` macro**

```rust
#[proc_macro]
pub fn use_route_context(input: TokenStream) -> TokenStream {
    // Expands to read context from RouterState
}
```

- [ ] **Step 4: Extend `define_route!` to accept `context`**

Parse `context: |args| { ... }` and store closure.

- [ ] **Step 5: Add demonstration in `example-app`**

`routes/admin/dashboard.rs` uses context for user permissions.

- [ ] **Step 6: Commit**

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs navi-macros/src/hooks.rs navi-macros/src/lib.rs example-app/src/routes/admin/dashboard.rs
git commit -m "feat: routeContext with use_route_context macro"
```

---

## Chunk 4: Macros Completeness – `use_navigate!`, `use_matched_route!`, Meta

### Task 4.1: `use_navigate!` Macro

**Files:**
- Modify: `navi-macros/src/hooks.rs`
- Modify: `navi-macros/src/lib.rs`

- [ ] **Step 1: Implement `use_navigate` macro**

```rust
pub fn use_navigate(_input: TokenStream) -> TokenStream {
    quote! {
        {
            let window_handle = cx.window_handle();
            ::navi_router::Navigator::new(window_handle)
        }
    }.into()
}
```

- [ ] **Step 2: Add to `lib.rs` exports**

- [ ] **Step 3: Commit**

```bash
git add navi-macros/src/hooks.rs navi-macros/src/lib.rs
git commit -m "feat: add use_navigate! macro"
```

---

### Task 4.2: `use_matched_route!` Macro

**Files:**
- Modify: `navi-macros/src/hooks.rs`

- [ ] **Step 1: Implement macro that returns current match info**

```rust
pub fn use_matched_route(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    quote! {
        {
            let state = ::navi_router::RouterState::global(cx);
            let current_match = state.current_match.as_ref()
                .expect("use_matched_route called but no route matched");
            // Return tuple (params, node)
            (current_match.0.clone(), current_match.1.clone())
        }
    }.into()
}
```

- [ ] **Step 2: Commit**

```bash
git add navi-macros/src/hooks.rs navi-macros/src/lib.rs
git commit -m "feat: add use_matched_route! macro"
```

---

### Task 4.3: Per‑Route `meta` Map

**Files:**
- Modify: `navi-router/src/route_tree.rs`
- Modify: `navi-macros/src/route.rs`

- [ ] **Step 1: Add `meta: HashMap<String, serde_json::Value>` to `RouteNode`**

- [ ] **Step 2: Extend macro to parse `meta: { ... }`**

Use a `syn::Expr` and evaluate to a `HashMap` at compile time or store as a closure that returns the map.

- [ ] **Step 3: Expose `current_meta()` on `RouterState`**

Iterate over matched route ancestors and leaf, merging their meta maps (child overrides parent).

- [ ] **Step 4: Update example root layout to display meta title**

- [ ] **Step 5: Commit**

```bash
git add navi-router/src/route_tree.rs navi-router/src/state.rs navi-macros/src/route.rs example-app/src/routes/__root.rs
git commit -m "feat: per-route meta map and RouterState::current_meta"
```

---

## Chunk 5: Async Blocker and `Link` Enhancements

### Task 5.1: Async Blocker Support

**Files:**
- Modify: `navi-router/src/blocker.rs`
- Modify: `navi-router/src/state.rs`

- [ ] **Step 1: Change `Blocker` predicate to return `BoxFuture<'static, bool>`**

Provide a synchronous convenience constructor that wraps a sync closure in `async move`.

- [ ] **Step 2: Update `RouterState::navigate` to await all blockers concurrently**

Use `futures::future::join_all`. If any returns `false`, abort navigation and store as pending.

- [ ] **Step 3: Add demonstration route with async confirmation dialog**

- [ ] **Step 4: Commit**

```bash
git add navi-router/src/blocker.rs navi-router/src/state.rs example-app/src/routes/blocking.rs
git commit -m "feat: async navigation blocker support"
```

---

### Task 5.2: `Link` – `activeOptions` and `resetScroll`

**Files:**
- Modify: `navi-router/src/components/link.rs`

- [ ] **Step 1: Add `ActiveOptions` struct and field to `Link`**

```rust
pub struct ActiveOptions {
    pub exact: bool,
    pub include_hash: bool,
    pub include_search: bool,
}
```

- [ ] **Step 2: Implement `active_options` builder method**

- [ ] **Step 3: Update `is_active` logic to respect options**

- [ ] **Step 4: Add `reset_scroll` field (default `true`) and pass to `NavigateOptions`**

- [ ] **Step 5: Implement preload cancellation for `PreloadType::Intent`**

Store the preload task handle in an element state and drop it on `MouseExit`.

- [ ] **Step 6: Commit**

```bash
git add navi-router/src/components/link.rs
git commit -m "feat: Link activeOptions, resetScroll, and preload cancellation"
```

---

## Chunk 6: Real Scroll Restoration and `Awaited` Component

### Task 6.1: Real Scroll Restoration

**Files:**
- Modify: `navi-router/src/components/scroll_restoration.rs`
- Modify: `example-app/src/routes/__root.rs`

- [ ] **Step 1: Change `ScrollRestoration` to accept `ScrollHandle`**

```rust
pub struct ScrollRestoration {
    scroll_handle: ScrollHandle,
}
```

- [ ] **Step 2: On route leave, save current offset to global map**

Use `cx.on_release` or `on_leave` hook equivalent.

- [ ] **Step 3: On route enter, restore offset after next frame**

```rust
cx.spawn(|mut cx| async move {
    cx.background_executor().timer(Duration::from_millis(0)).await;
    cx.update(|cx| {
        // restore scroll_handle.set_offset()
    }).ok();
}).detach();
```

- [ ] **Step 4: Update example app to use `ScrollRestoration` with main scroll handle**

- [ ] **Step 5: Commit**

```bash
git add navi-router/src/components/scroll_restoration.rs example-app/src/routes/__root.rs
git commit -m "feat: real scroll restoration with ScrollHandle"
```

---

### Task 6.2: `Awaited` Component

**Files:**
- Create: `navi-router/src/components/awaited.rs`
- Modify: `navi-router/src/components/mod.rs`

- [ ] **Step 1: Define `Awaited` element with fallback and child**

```rust
pub struct Awaited<R: RouteDef> {
    fallback: Option<Box<dyn Fn() -> AnyElement>>,
    child: Option<Box<dyn Fn(R::LoaderData) -> AnyElement>>,
    _phantom: PhantomData<R>,
}
```

- [ ] **Step 2: Implement `Element` trait to render fallback until loader data ready**

Read `RouterState::global(cx).get_loader_data::<R>()`.

- [ ] **Step 3: Provide builder API**

- [ ] **Step 4: Add example route `awaited.rs`**

- [ ] **Step 5: Commit**

```bash
git add navi-router/src/components/awaited.rs navi-router/src/components/mod.rs example-app/src/routes/awaited.rs
git commit -m "feat: add Awaited component for granular data-ready rendering"
```

---

## Chunk 7: Codegen Improvements – Pathless Layouts, Route Groups, Optional Segments

### Task 7.1: Pathless Layout Directories (`_` prefix)

**Files:**
- Modify: `navi-codegen/src/scanner.rs`

- [ ] **Step 1: Detect directories starting with `_`**

Treat them as layout routes that do not add a URL segment.

- [ ] **Step 2: Update `file_name_to_pattern` to skip `_` dir segments**

- [ ] **Step 3: Ensure children correctly inherit the layout as parent**

- [ ] **Step 4: Add example `routes/_auth/` directory**

- [ ] **Step 5: Commit**

```bash
git add navi-codegen/src/scanner.rs example-app/src/routes/_auth/
git commit -m "feat: pathless layout directories with _ prefix"
```

---

### Task 7.2: Route Groups `(group)` – Validation

**Files:**
- Modify: `navi-codegen/src/scanner.rs`

- [ ] **Step 1: Add duplicate path detection after scanning**

Collect all route patterns and check for conflicts. Emit a compile error if duplicates found.

- [ ] **Step 2: Skip `(group)` dir segments in URL pattern generation**

- [ ] **Step 3: Commit**

```bash
git add navi-codegen/src/scanner.rs
git commit -m "feat: route group validation and duplicate path detection"
```

---

### Task 7.3: Optional Segment File Names `{-$param}.rs`

**Files:**
- Modify: `navi-codegen/src/scanner.rs`

- [ ] **Step 1: Parse filenames matching `{-$...}.rs`**

Map to `Segment::Optional`.

- [ ] **Step 2: Update `file_name_to_pattern` accordingly**

- [ ] **Step 3: Add example route `routes/users/{-$id}.rs`**

- [ ] **Step 4: Commit**

```bash
git add navi-codegen/src/scanner.rs example-app/src/routes/users/{-$id}.rs
git commit -m "feat: support optional path segments with {-$param}.rs"
```

---

### Task 7.4: Emit New Fields in Generated Code

**Files:**
- Modify: `navi-codegen/src/generator.rs`

- [ ] **Step 1: Parse `loaderDeps`, `staleTime`, `beforeLoad`, `meta` from route file content**

Extend `parse_route_file` to scan for these attributes using regex.

- [ ] **Step 2: Emit corresponding assignments in `build_route_tree`**

- [ ] **Step 3: Commit**

```bash
git add navi-codegen/src/generator.rs navi-codegen/src/scanner.rs
git commit -m "feat: emit loaderDeps, staleTime, beforeLoad, meta in generated route tree"
```

---

## Chunk 8: Devtools Expansion

### Task 8.1: Route Tree Inspector Tab

**Files:**
- Modify: `navi-devtools/src/lib.rs`
- Create: `navi-devtools/src/routes_panel.rs`

- [ ] **Step 1: Add `DevtoolsTab::Routes` enum variant**

- [ ] **Step 2: Implement `RoutesPanel` view that reads `RouterState` and renders tree**

Use GPUI's `uniform_list` or recursive `div`s.

- [ ] **Step 3: Highlight current match and show params**

- [ ] **Step 4: Integrate into main devtools UI**

- [ ] **Step 5: Commit**

```bash
git add navi-devtools/src/lib.rs navi-devtools/src/routes_panel.rs
git commit -m "feat: add route tree inspector tab to devtools"
```

---

### Task 8.2: Navigation Timeline UI

**Files:**
- Modify: `navi-devtools/src/timeline.rs`
- Modify: `navi-devtools/src/lib.rs`

- [ ] **Step 1: Store timeline events in a global list (e.g., `Vec<LoggedEvent>`)**

- [ ] **Step 2: Create `TimelinePanel` view that renders scrollable list of events**

- [ ] **Step 3: Add tab to devtools**

- [ ] **Step 4: Commit**

```bash
git add navi-devtools/src/timeline.rs navi-devtools/src/lib.rs
git commit -m "feat: navigation timeline UI in devtools"
```

---

### Task 8.3: Search Params Inspector

**Files:**
- Modify: `navi-devtools/src/lib.rs`

- [ ] **Step 1: Add a collapsible JSON tree displaying `RouterState::current_location().search`**

- [ ] **Step 2: Update when location changes (via subscription)**

- [ ] **Step 3: Commit**

```bash
git add navi-devtools/src/lib.rs
git commit -m "feat: search params inspector in devtools"
```

---

## Chunk 9: Example App Overhaul

### Task 9.1: Add All Demonstration Routes

**Files:**
- Create multiple route files in `example-app/src/routes/`

- [ ] **Step 1: Create each route as specified in the enhancement plan**

- `routes/login.rs`
- `routes/admin/dashboard.rs`
- `routes/posts/mod.rs`, `routes/posts/index.rs`, `routes/posts/$id.rs`
- `routes/lifecycle.rs`
- `routes/blocking.rs`
- `routes/awaited.rs`
- `routes/scroll.rs`
- `routes/meta.rs`

- [ ] **Step 2: Update `routes/__root.rs` with navigation bar, title label, scroll restoration**

- [ ] **Step 3: Update `main.rs` to pass `RouterOptions`**

- [ ] **Step 4: Ensure all features are demonstrable**

- [ ] **Step 5: Commit**

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

- [ ] **Step 1: Write tests for matcher optional segments, splat, ranking**

- [ ] **Step 2: Write tests for `beforeLoad` and loader interception**

- [ ] **Step 3: Write tests for async blocker**

- [ ] **Step 4: Write tests for validation backends**

- [ ] **Step 5: Run tests and ensure pass**

- [ ] **Step 6: Commit**

```bash
git add navi-router/src/
git commit -m "test: add unit tests for new router features"
```

---

### Task 10.2: Integration Test Crate

**Files:**
- Create: `navi-test/` crate

- [ ] **Step 1: Add `navi-test` to workspace**

- [ ] **Step 2: Write headless GPUI test that builds route tree and navigates**

- [ ] **Step 3: Assert loader caching and invalidation**

- [ ] **Step 4: Commit**

```bash
git add navi-test/ Cargo.toml
git commit -m "test: add integration test crate for navi-router"
```

---

### Task 10.3: Update README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Rewrite README with feature parity checklist and code snippets**

- [ ] **Step 2: Include quick start, configuration, and API reference**

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: rewrite README with full feature documentation"
```

---

## Implementation Order

1. **Chunk 1:** `beforeLoad` hooks, loader outcome enum, `onEnter`/`onLeave`
2. **Chunk 2:** `loaderDeps`, `staleTime`/`gcTime`, suspense timing
3. **Chunk 3:** `notFoundMode`, 404 routes, route context
4. **Chunk 4:** `use_navigate!`, `use_matched_route!`, meta
5. **Chunk 5:** Async blocker, `Link` enhancements
6. **Chunk 6:** Scroll restoration, `Awaited` component
7. **Chunk 7:** Codegen improvements
8. **Chunk 8:** Devtools expansion
9. **Chunk 9:** Example app overhaul
10. **Chunk 10:** Testing and documentation

Each chunk should be reviewed by the plan-document-reviewer subagent after writing, before proceeding to the next chunk.

---

**Plan complete and saved to `docs/superpowers/plans/2026-04-19-navi-router-enhancement.md`. Ready to execute?**
```
