<div align="center">
  <img src="docs/logo.png" alt="Navi Router Logo" width="300"/>
  <p>
    <strong>A powerful file‑based router for GPUI</strong><br/>
    Featuring loaders, suspense, devtools, and full TanStack Router feature parity.
  </p>
  <p>
    <img src="https://img.shields.io/badge/Rust-1.99%20%7C%20edition%202024-000000?style=flat-square&logo=rust" alt="Rust"/>
    <img src="https://img.shields.io/badge/GPUI-0.2-000000?style=flat-square" alt="GPUI"/>
    <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License MIT"/>
    <a href=".github/workflows/ci.yml"><img src="https://img.shields.io/badge/CI-clippy%20pedantic%20%2B%20tests-brightgreen?style=flat-square&logo=githubactions" alt="CI"/></a>
  </p>
</div>

---

## Features

- **File‑based routing**: Define routes intuitively via the filesystem.
- **Nested layouts**: Hierarchical layouts powered by `<Outlet>`.
- **Loaders**: Built-in data fetching with caching, stale-time, and garbage collection.
- **Suspense boundaries**: Granular loading states using `Awaited`.
- **`beforeLoad` hooks**: Robust route guards with redirect/notFound support.
- **Navigation blockers**: Async/sync blockers to prevent data loss from unsaved changes.
- **Scroll restoration**: Automatic memory and restoration of scroll positions,
  keyed by path + normalized query, capacity-bounded, opt-out via
  `scroll_restoration: false`, with per-navigation `reset_scroll`.
- **Navigation transactions**: Batch `Push`/`Replace`/`Back`/`Forward`/`Go`
  (`NavOp`) applied atomically with a single re-render (`Navigator::transact`).
- **External links**: `http(s):`, `mailto:` and `tel:` hrefs on `<Link>` are
  handed to the OS instead of the router.
- **Devtools**: Inspect routes, cache, timeline, and state directly in your app.
- **Validation**: Seamless integration with `validator`, `garde`, `validify`, and `valico`.
- **Nested search params** (`nested-search` feature): typed, bracket-nested
  query strings (`filter[status]=open&tags[0]=rust`) via `serde_qs`.
- **Codegen**: Automatic route tree generation from `src/routes`.
- **Type‑safe hooks**: Macros like `use_params!`, `use_search!`, and `use_loader_data!`.
- **Deterministic matching**: Static segments always win over dynamic ones,
  with a ranked radix trie and structural route-tree validation.
- **Link preloading**: `Render` / `Intent` (hover + delay) / `Viewport`
  strategies with per-href de-duplication.
- **Event timeline**: `BeforeNavigate → BeforeLoad → Resolved` events feed the
  devtools timeline and subscriber callbacks.

## Quick Start

### 1. Add Dependencies

Add `navi-router` and `navi-macros` to your `Cargo.toml`:

```toml
[dependencies]
navi-router = { path = "navi-router" }
navi-macros = { path = "navi-macros" }
```

### 2. Define Routes

Create a root layout (`src/routes/__root.rs`):

```rust
use gpui::*;
use navi_macros::define_route;
use navi_router::components::{Link, Outlet};

#[derive(Clone, IntoElement)]
struct RootLayout;

impl RenderOnce for RootLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .child(Link::new("/").child("Home"))
            .child(Outlet::new())
    }
}

define_route!(RootRoute, path: "/", is_layout: true, component: RootLayout);
```

Add an index route (`src/routes/index.rs`):

```rust
use gpui::*;
use navi_macros::define_route;

#[derive(Clone, IntoElement)]
struct HomePage;

impl RenderOnce for HomePage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child("Welcome!")
    }
}

define_route!(IndexRoute, path: "/", is_index: true, component: HomePage);
```

### 3. Configure Code Generation

Set up `build.rs` to generate the route tree:

```rust
fn main() {
    let config = navi_codegen::NaviConfig::from_file("navi.config.json")
        .expect("Failed to read config");
    navi_codegen::generator::write_route_tree(&config).unwrap();
}
```

> [!NOTE]
> Ensure `navi.config.json` is present in your project root (see [Configuration](#configuration)).

### 4. Initialize the Router

In your `main.rs`:

```rust
use navi_router::{RouterProvider, Location, RouterOptions, NotFoundMode};

mod route_tree { include!("route_tree.gen.rs"); }

let tree = route_tree::build_route_tree();
let router = RouterProvider::new_with_options(
    window_id,
    window_handle,
    Location::new("/"),
    tree,
    RouterOptions {
        not_found_mode: NotFoundMode::Fuzzy,
        ..Default::default()
    },
    cx,
);
route_tree::register_routes(cx);
```

## Configuration

Control code generation behavior via `navi.config.json`:

```json
{
  "routes_directory": "./src/routes",
  "generated_route_tree": "./src/route_tree.gen.rs",
  "route_token": "route",
  "index_token": "index",
  "route_file_ignore_prefix": "-"
}
```

## Route Definition API

The `define_route!` macro provides a comprehensive, type-safe way to configure routes:

```rust
define_route!(
    MyRoute,
    path: "/users/$id",
    params: UserParams,
    search: UserSearch,
    data: User,
    loader: |params, executor| async move { /* fetch data */ },
    before_load: |ctx| async move { BeforeLoadResult::Ok },
    on_enter: |loc| log::info!("entered"),
    on_leave: |loc| log::info!("left"),
    stale_time: Duration::from_secs(60),
    gc_time: Duration::from_secs(300),
    meta: { /* custom metadata */ },
    component: MyComponent,
);
```

## Hooks & Components

### Hooks
| Macro | Description |
| :--- | :--- |
| `use_params!(MyRoute)` | Extracts typed path parameters. |
| `use_search!(MyRoute)` | Extracts typed search/query parameters. |
| `use_loader_data!(MyRoute)` | Accesses data returned by the route's loader. |
| `use_navigate!()` | Programmatically navigate between routes. |
| `use_matched_route!(MyRoute)` | Gets context about the currently matched route. |
| `use_route_context!(MyRoute)` | Accesses the full route context. |

### Components
| Component | Description |
| :--- | :--- |
| `Link` | Navigation links with built-in active styling. |
| `Outlet` | Renders the matched child route within a layout. |
| `SuspenseBoundary` | Displays a fallback UI while nested routes load. |
| `Awaited` | Waits for and renders loader data. |
| `ScrollRestoration` | Automatically saves and restores scroll positions. |

## Devtools

> [!TIP]
> Press `Cmd+Shift+D` (or `Ctrl+Shift+D` on Windows/Linux) to open the Devtools panel.

The Devtools interface provides four main tabs:
- **Routes**: Visual tree view and test navigation.
- **Cache**: Inspect and invalidate the `rs-query` cache.
- **Timeline**: Event log with search, filter, and export capabilities.
- **State**: Overview of router state, navigation blockers, and metadata.

## Feature Comparison

| Feature | Navi Router | TanStack Router |
| :--- | :---: | :---: |
| File‑based routing | ✅ | ✅ |
| Nested layouts | ✅ | ✅ |
| Loaders & Caching | ✅ | ✅ |
| `beforeLoad` guards | ✅ | ✅ |
| Async navigation blockers | ✅ | ✅ |
| Scroll restoration | ✅ | ✅ |
| Built-in Devtools | ✅ | ✅ |
| Type‑safe search params | ✅ | ✅ |
| Route context | ✅ | ✅ |
| Deterministic route ranking | ✅ | ✅ |
| Link preloading (render/intent) | ✅ | ✅ |

## Development

```sh
just test        # workspace tests
just clippy      # workspace lints (pedantic), zero warnings, per optional feature
just fmt         # rustfmt check
just verify      # fmt + clippy + tests
just coverage    # cargo-llvm-cov summary
cargo bench --bench core --bench adapter   # criterion benchmarks
```

## Performance

The router is engineered for allocation-free hot paths — see
[docs/PERFORMANCE.md](docs/PERFORMANCE.md) for the design and
[docs/BENCHMARKS.md](docs/BENCHMARKS.md) for full before/after numbers:

- **Zero-copy route matching**: stack-collected parameter bindings, interned
  `Arc<str>` segment names, FxHash static-child lookup, `HashMap` materialized
  once on success (matching is 14–38% faster across the board).
- **Per-frame paths allocate nothing**: `use_params!`/`use_search!`/
  `use_loader_data!` read navigation-time caches; `Link` active checks are
  branch-and-byte comparisons (−94%); loader `QueryKey`s are memoized per
  navigation.
- **Bounded memory everywhere**: ring-buffered event log, capped preload
  registry, O(1) scroll-eviction, `Arc<RouteTree>` shared between the core
  and the adapter (no deep tree clone at startup).
- **Lock-free single-owner history**: no `Mutex` in the navigation path
  (back/forward is −67%).

Quality gates (also enforced in CI):

- `cargo clippy --workspace --all-targets -- -W clippy::pedantic -D warnings`
  — zero warnings, no `#[allow]` attributes in the codebase.
- The four validation integrations (`validator`, `garde`, `validify`,
  `valico`) are mutually exclusive blanket impls, so they are linted and
  tested **per feature** instead of under `--all-features`.
- `clippy.toml` calibrates a few shape thresholds for the devtools UI module
  (large render methods) instead of scattering allow attributes.

The dependency stack is the post-restructure `gpui-component` 0.8 /
`gpui-kit` release line, with GPUI supplied by the matching `gpui-pre`
snapshot (recent zed main) and `rs-query` pinned to a compatible commit.

See [docs/BRAINSTORM.md](docs/BRAINSTORM.md) for the full improvement
brainstorm (performance, DRY/KISS, features) and the remaining follow-ups
(deep-links re-introduction, coverage gate).
