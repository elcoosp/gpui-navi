Now I have a thorough picture of the entire codebase. Let me produce the comprehensive plan.Now I have everything I need. Let me produce the full implementation plan.Here is the complete, phased plan. I've read every file carefully — the architecture is sound but `navi-codegen` is the weak link blocking everything else.

---

## Phase 1 — Fix file-based routing in `navi-codegen` (the foundation)

### 1.1 `$` in filenames — make it work end-to-end

The current `sanitize_module_name` replaces `$` with `_dollar_`, which produces module paths like `users::_dollar_id`. That path doesn't exist — you'd have to literally name the file `_dollar_id.rs`. The real fix is a **two-step sanitize**:

```
$id.rs  →  module name: param_id   (Rust identifier, valid)
           route segment: $id      (kept as-is for pattern matching)
```

In `scanner.rs`, split the concern:
- `file_name_to_segment(name)` — keeps `$` intact → produces `$id` for the pattern
- `file_stem_to_module_ident(name)` — strips leading `$`, produces `id` or prefixes with `param_` if it would conflict

Then `build_module_path` uses the sanitized ident, while `file_name_to_pattern` uses the raw segment. Currently they share a single path — split them.

### 1.2 Parent/child hierarchy — actually emit it

Right now the generator emits a flat list of `tree.add_route(X::build_node())` calls with no `parent` field set. TanStack Router's file conventions encode hierarchy via **directory nesting**:

```
routes/
  __root.rs           → id: __root,      parent: None
  index.rs            → id: index,       parent: __root
  users.rs            → id: users,       parent: __root    (layout if has Outlet)
  users/
    index.rs          → id: users_index, parent: users
    $id.rs            → id: users_$id,   parent: users
  (auth)/             → pathless group, no URL segment
    login.rs          → id: login,       parent: __root
```

The `compute_parent` function already tries this but emits a raw module path, not a route id. Fix: after building the full `Vec<RouteInfo>`, do a second pass that:
1. Sorts by path depth ascending
2. For each route, finds the parent by matching the longest common directory prefix
3. Sets `parent` to the parent's `id` (not module path)

### 1.3 Layout vs index vs regular — detect reliably

Adopt TanStack Router's exact conventions:

| Filename | Meaning |
|---|---|
| `__root.rs` | Root layout, parent of everything |
| `_layout.rs` or `_name.rs` (leading `_`) | Layout route — has `<Outlet>`, children render inside it |
| `index.rs` | Index route — renders at parent's exact path |
| `$param.rs` | Dynamic segment |
| `$.rs` | Splat/catch-all |
| `(group)/` | Pathless directory — groups routes, no URL contribution |
| `-ignored.rs` | Prefix `-` → skip file |

`is_layout` should be inferred: a file is a layout if it shares a path prefix with other files (i.e., there's a same-named directory alongside it). E.g. `users.rs` + `users/` directory → `users.rs` is a layout.

### 1.4 Generator output — emit proper stubs

Current output is broken: `define_route!(SomeName, path: "...");` in the generated file, then `use crate::routes::some::module as SomeName` — but the route file doesn't actually contain this struct, so it fails to compile.

**Target output per route:**

```rust
// In route_tree.gen.rs
pub mod routes {
    pub mod users {
        pub mod _dollar_id {
            use navi_macros::define_route;
            // params struct stub — user fills in fields
            #[derive(Clone, Debug, Default, serde::Deserialize)]
            pub struct Params { pub id: String }
            
            define_route!(
                UsersDollarIdRoute,
                path: "/users/$id",
                params: Params,
                is_index: false,
            );
        }
    }
}

pub fn build_route_tree() -> navi_router::RouteTree {
    let mut tree = navi_router::RouteTree::new();
    
    // __root
    tree.add_route(navi_router::RouteNode {
        id: "__root__".into(),
        pattern: navi_router::RoutePattern::parse("/"),
        parent: None,
        is_layout: true,
        is_index: false,
        has_loader: false,
        loader_stale_time: None,
        loader_gc_time: None,
        preload_stale_time: None,
    });
    
    // users/$id
    {
        let mut node = routes::users::_dollar_id::UsersDollarIdRoute::build_node();
        node.parent = Some("users".into());
        tree.add_route(node);
    }
    tree
}
```

The stubs are **compilable and overrideable** — user edits the params struct in place, or better, moves it to their own file and re-exports.

### 1.5 Fix `build.rs` in `example-app`

Currently commented out. Uncomment, wire to `NaviConfig::from_file("navi.config.json")`, add `println!("cargo:rerun-if-changed=src/routes")` so Cargo reacts to file changes.

---

## Phase 2 — Full TanStack Router file convention parity

### 2.1 All segment types — table

| File/dir name | URL segment emitted | Rust pattern | Notes |
|---|---|---|---|
| `about.rs` | `/about` | `Static("about")` | ✅ works |
| `$id.rs` | `/$id` → `$id` | `Dynamic{name:"id"}` | needs § 1.1 fix |
| `$.rs` | `/$` → splat | `Splat` | ✅ works |
| `{$id}.ext.rs` | `/{$id}.ext` | `Dynamic{suffix:".ext"}` | ✅ pattern exists, need file→seg fix |
| `{-$id}.rs` | optional | `Optional{name:"id"}` | ✅ pattern exists |
| `(auth)/` | *(none)* | pathless group | needs directory skip |
| `__root.rs` | `/` | root | ✅ |
| `index.rs` | exact parent path | index | ✅ |
| `_layout.rs` | exact parent path | layout | needs detection |

### 2.2 Pathless route groups `(group)/`

Directories wrapped in `()` contribute no URL segment, but logically group routes under a shared layout. In `file_name_to_pattern`, already skipped (`starts_with('(')` check). But: group directories need their own layout file `(auth)/_layout.rs` or `(auth).rs` — emit a `RouteNode` with `is_layout: true` and no added segment.

### 2.3 Named splats

`$.rs` → param name `*splat`. But TanStack also supports `$name.rs` where the whole filename is used as the splat name for the rest. Emit `Splat` with the name stored so `use_params!` can retrieve it.

### 2.4 Route sorting / ranking

TanStack Router ranks routes: static > dynamic > optional > splat, then by depth. Current `scanner.rs` sorts by `/` count but ignores segment type. Fix `compute_rank` to apply: `depth × 1000 + static_count × 100 + dynamic_count × 10 - splat × 500`.

---

## Phase 3 — `navi-macros` improvements for great DX

### 3.1 `define_route!` — make `component` meaningful

Currently `component_ty` is parsed but `#[allow(dead_code)]`. Wire it:

```rust
// Generated in define_route! expansion:
impl #name {
    pub fn register(cx: &mut gpui::App) {
        navi_router::components::outlet::register_route_component(
            Self::name(),
            |cx| gpui::Component::new(#component_ty).into_any_element(),
        );
        Self::register_loader(cx);
    }
}
```

Now user calls `UserDetailRoute::register(cx)` instead of two separate calls. No more manual `register_route_component` at the bottom of `main`.

### 3.2 `define_route!` — `parent` key

Add an optional `parent: "route_id"` key so macros can set parent explicitly, avoiding the need to mutate `node.parent` at the call site:

```rust
define_route!(
    UserDetailRoute,
    path: "/users/$id",
    parent: "users",
    params: UserParams,
    loader: |...| async { ... },
    component: UserDetailPage,
);
```

### 3.3 `use_params!` — fix the `cx` dependency

Currently requires `cx: &mut App` in scope. Make it work with just a reference by reading from the global without mutation:

```rust
macro_rules! use_params {
    ($route_ty:ty, $cx:expr) => { ... }
}
// Or keep the single-arg form but document the `cx` requirement clearly.
```

### 3.4 `use_navigate!` macro — remove `cx.window_handle()` requirement

```rust
pub fn use_navigate(input: TokenStream) -> TokenStream {
    // Accept optional window handle arg OR derive from window context
    let expanded = quote! {
        Navigator::new(_window.window_handle())
    };
}
```

### 3.5 New macro: `use_match!`

```rust
// Returns the matched RouteNode id and params
let (route_id, params) = use_match!(cx);
```

---

## Phase 4 — Outlet rendering — nested layouts

This is the most important functional gap. Currently `Outlet` renders only the leaf matched route. It needs to render the **full ancestor chain**.

### 4.1 Route ancestry resolution

Add to `RouteTree`:

```rust
pub fn ancestors(&self, route_id: &str) -> Vec<&RouteNode> {
    let mut chain = Vec::new();
    let mut current = route_id;
    while let Some(node) = self.get_node(current) {
        chain.push(node);
        match &node.parent {
            Some(p) => current = p,
            None => break,
        }
    }
    chain.reverse();
    chain
}
```

### 4.2 Outlet depth tracking

`Outlet` needs to know its depth in the render tree to render the right ancestor. Add a `depth` field, defaulting to auto-detect via a context value pushed/popped as the render tree descends:

```rust
// RouterProvider pushes depth=0 into context
// Each Outlet increments: renders ancestors[depth], pushes depth+1
// Child Outlet renders ancestors[depth+1], and so on
```

Store the current outlet depth in `navi_core::context` keyed by `OutletDepth(usize)`.

### 4.3 `RouterProviderWithChildren` — push route context per layout

As each layout renders, push its params into `navi_core::context` so child routes can consume them via `use_params!` regardless of nesting depth.

---

## Phase 5 — Loader DX improvements

### 5.1 Stale-while-revalidate

Currently cache is checked by `route_id:params_json` key but never invalidated. Add `loader_stale_time` expiry:

```rust
pub struct CacheEntry {
    pub data: Arc<dyn Any + Send + Sync>,
    pub inserted_at: std::time::Instant,
}
```

In `trigger_loader_with_locations`, check `inserted_at.elapsed() > node.loader_stale_time` before returning a cache hit.

### 5.2 `use_loader_data!` — don't trigger loader from render

The current macro calls `trigger_loader` if data is absent, from inside a render call. This is a side effect during rendering — GPUI doesn't guarantee render is called once. Instead:

- Trigger loader in `navigate()` when `has_loader` is true (already done)
- `use_loader_data!` should be **read-only** — return `None` while loading, `Some(data)` when ready
- Remove the `trigger_loader` call from the macro expansion entirely

### 5.3 Pending/error states

Add `LoaderState` enum to `RouterState`:

```rust
pub enum LoaderState {
    Idle,
    Loading { route_id: String },
    Error { route_id: String, message: String },
}
```

Expose via `use_loader_state!(RouteType)` macro.

---

## Phase 6 — Navigation completeness

### 6.1 `Navigator::preload`

```rust
pub fn preload(&self, path: impl Into<String>, cx: &mut App) {
    // Runs loader in background, stores in cache, does NOT navigate
}
```

Link component already has a `preload` field — wire it to call this on hover/viewport.

### 6.2 `Link` active state styling — use GPUI styles, not string children

Current active/inactive class appends string children (broken). Replace with:

```rust
let active_style = /* closure that applies Tailwind-like styles */;
element = if is_active { element.bg(...).text_color(...) } else { element };
```

Or accept a `active_props: impl Fn(Div) -> Div` closure.

### 6.3 `use_can_go_back!` — actually implement

Currently returns `false`. Wire to `RouterState::global(cx).history.can_go_back()`.

### 6.4 `NavigateOptions` — honor `reset_scroll`

In `navigate()`, after history push, if `reset_scroll == Some(true)`, broadcast a scroll-reset event via the event bus.

---

## Phase 7 — `navi-codegen` CLI tool

Add a `[[bin]]` target to `navi-codegen/Cargo.toml`:

```toml
[[bin]]
name = "navi-codegen"
path = "src/main.rs"
```

```rust
// src/main.rs
fn main() {
    let config = NaviConfig::from_file("navi.config.json")
        .unwrap_or_default();
    navi_codegen::generator::write_route_tree(&config)
        .expect("codegen failed");
    println!("Generated {}", config.generated_route_tree);
}
```

This lets users run `cargo run -p navi-codegen` directly during development without a build script, and enables watch-mode integration with `bacon`.

---

## Concrete file-by-file changes summary

| File | Changes |
|---|---|
| `navi-codegen/src/scanner.rs` | Split `sanitize_module_name` from `file_stem_to_segment`; parent inference second-pass; `$` → `param_` ident; group dir detection; layout inference from sibling dir |
| `navi-codegen/src/generator.rs` | Emit mod hierarchy; per-route `Params` stubs; parent field wiring; register stubs; `build_route_tree` with full hierarchy |
| `navi-codegen/src/config.rs` | Add `emit_stubs: bool` option; `watch: bool` for future use |
| `navi-codegen/src/main.rs` | New CLI entry point |
| `navi-macros/src/route.rs` | Add `parent` key; wire `component` to `register()`; fix `register_loader` to call `register_route_component` |
| `navi-macros/src/hooks.rs` | Remove `trigger_loader` side-effect from `use_loader_data!`; implement `use_can_go_back!` |
| `navi-router/src/route_tree.rs` | Add `ancestors()` method; fix rank formula |
| `navi-router/src/state.rs` | Add `LoaderState` enum; `CacheEntry` with timestamp; stale check |
| `navi-router/src/components/outlet.rs` | Depth-aware ancestor rendering |
| `navi-router/src/components/link.rs` | Fix active state to use style methods not string children |
| `navi-router/src/navigator.rs` | Add `preload()` |
| `example-app/build.rs` | Uncomment and wire to `NaviConfig::from_file` |

---

## Suggested implementation order

1. **§ 1.1 + 1.2 + 1.3** — fix scanner, get `$` files and parent hierarchy correct. This is the blocker for everything else. Write a unit test for `parse_route_file` that covers `$id.rs`, `(auth)/login.rs`, `users/_layout.rs`.
2. **§ 1.4** — fix generator output to be compilable. Run `cargo check -p example-app` after this to verify.
3. **§ 1.5** — uncomment `build.rs`, run codegen end-to-end.
4. **§ 3.1** — wire `component` key in `define_route!` to auto-register, removing the manual `register_route_component` boilerplate from `main.rs`.
5. **§ 4.1 + 4.2** — nested Outlet rendering. This is what makes layouts actually work.
6. **§ 5.2** — remove the loader trigger from render. Then **§ 5.1** stale-while-revalidate.
7. **§ 6** — navigation polish.
8. **§ 7** — CLI binary, tie it to `bacon`.

The existing architecture — `RoutePattern`, `RouteMatcher`, `History`, `RouterState`, `LoaderRegistry`, `ContextTree` — is all solid. The codegen and macro wiring are the only things standing between the current state and a genuinely usable TanStack Router port.
