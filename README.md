<div align="center">
  <img src="https://cdn.jsdelivr.net/gh/elcoosp/gpui-navi@main/docs/logo.png" alt="Navi Logo" width="300"/>
  <p>
    <strong>A powerful file‑based router for Rust UI frameworks</strong><br/>
    Framework‑agnostic by design — one route table, every frontend, powered by <a href="https://github.com/elcoosp/quoin">quoin</a>.<br/>
    Featuring loaders, suspense, devtools, and full TanStack Router feature parity.
  </p>
  <p>
    <img src="https://img.shields.io/badge/Rust-1.99%20%7C%20edition%202024-000000?style=flat-square&logo=rust" alt="Rust"/>
    <img src="https://img.shields.io/badge/quoin-latest%20main-5C2D91?style=flat-square" alt="quoin"/>
    <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License MIT"/>
    <a href=".github/workflows/ci.yml"><img src="https://img.shields.io/badge/CI-clippy%20pedantic%20%2B%20tests-brightgreen?style=flat-square&logo=githubactions" alt="CI"/></a>
  </p>
</div>

---

Navi is a file‑based router for Rust UI applications. It is **not tied to any
single UI framework**: the routing engine, loaders, guards, cache and devtools
live in framework‑free crates, and all rendering seams are delegated to
[quoin](https://github.com/elcoosp/quoin) — one reactive core, many frontend
backends. The same route table, components and devtools panel run unmodified
on GPUI, Dioxus, Leptos, Xilem and Floem.

## Frameworks

| Framework | Backend | Status | Notes |
| :--- | :--- | :--- | :--- |
| **GPUI** | `quoin` `gpui` feature | ✅ first‑class | Devtools panel, keybindings, `run_app!` host |
| **Dioxus** | `quoin` `dioxus` feature | ✅ first‑class | Dioxus desktop (wry/tao window) |
| **Leptos** | `quoin` `leptos` feature | ✅ first‑class | CSR, `wasm32-unknown-unknown` |
| **Xilem** | `quoin` `xilem` feature | ✅ first‑class | Xilem main, rebuild‑pump refresh |
| **Floem** | `quoin` `floem` feature | ✅ first‑class | Floem main, reactive bridge |

Selecting a framework is a feature flag on your crate — see
[Quick Start](#quick-start). There is nothing GPUI‑specific inside the router
itself; GPUI subtrees remain reachable through quoin's `rsx!` escape hatch
(`gpui-rsx-classes` is enabled automatically with the `gpui` feature).

## Features

- **Framework‑agnostic core**: routing, loaders, cache, guards and devtools
  never touch a UI framework — quoin renders the views.
- **File‑based routing**: define routes intuitively via the filesystem.
- **Nested layouts**: hierarchical layouts powered by `<Outlet>`.
- **Loaders**: built‑in data fetching with caching, stale‑time, and garbage
  collection (backed by [`rs-query`](https://github.com/elcoosp/rs-query)).
- **Suspense boundaries**: granular loading states using `Awaited`.
- **`beforeLoad` hooks**: robust route guards with redirect/notFound support.
- **Navigation blockers**: async/sync blockers to prevent data loss from
  unsaved changes.
- **Scroll restoration**: automatic memory and restoration of scroll
  positions, keyed by path + normalized query, capacity‑bounded, opt‑out via
  `scroll_restoration: false`, with per‑navigation `reset_scroll`.
- **Navigation transactions**: batch `Push`/`Replace`/`Back`/`Forward`/`Go`
  (`NavOp`) applied atomically with a single re‑render
  (`Navigator::transact`).
- **External links**: `http(s):`, `mailto:` and `tel:` hrefs on `<Link>` are
  handed to the OS instead of the router.
- **Devtools**: inspect routes, cache, timeline, and state directly in your
  app — on every supported framework.
- **Validation**: seamless integration with `validator`, `garde`, `validify`,
  and `valico`.
- **Nested search params** (`nested-search` feature): typed, bracket‑nested
  query strings (`filter[status]=open&tags[0]=rust`) via `serde_qs`.
- **Codegen**: automatic route tree generation from `src/routes`.
- **Type‑safe hooks**: macros like `use_params!`, `use_search!`, and
  `use_loader_data!`.
- **Deterministic matching**: static segments always win over dynamic ones,
  with a ranked radix trie and structural route‑tree validation.
- **Link preloading**: `Render` / `Intent` (hover + delay) / `Viewport`
  strategies with per‑href de‑duplication.
- **Event timeline**: `BeforeNavigate → BeforeLoad → Resolved` events feed
  the devtools timeline and subscriber callbacks.

## Workspace Crates

| Crate | Purpose |
| :--- | :--- |
| `navi-core` | Shared primitives (window keys, ids, config types). |
| `navi-router-core` | Pure routing engine: patterns, ranked radix trie matching, route‑tree validation. No UI, no async runtime. |
| `navi-router` | Router runtime: `Navigator`, `RouterProvider`, components (`Link`, `Outlet`, `Awaited`, …), blockers, scroll restoration. |
| `navi-macros` | `define_route!`, `use_params!`, `use_search!`, `use_loader_data!`, … |
| `navi-codegen` | Build‑time route‑tree generation from `src/routes` (`navi.config.json`). |
| `navi-devtools` | Framework‑agnostic devtools model (routes, cache, timeline, state). |
| `navi-test` | Test utilities for router‑based applications. |

All rendering goes through [quoin](https://github.com/elcoosp/quoin): route UI
is expressed as `quoin_ui::Node` trees (or `component!` shells), and quoin's
macro emitters lower them to the selected framework.

## Quick Start

### 1. Add Dependencies

Add the Navi crates plus quoin with **your** framework's feature:

```toml
[dependencies]
navi-router = { path = "navi-router" }
navi-macros = { path = "navi-macros" }
quoin      = { path = "quoin/quoin", features = ["dioxus"] } # or gpui / leptos / xilem / floem
```

### 2. Define Routes

Routes render as `quoin_ui::Node` trees — plain data any backend understands
(`src/routes/index.rs`):

```rust
use navi_macros::define_route;
use navi_router::components::Link;
use quoin_ui::{Node, StyleCmd};

fn index_node() -> Node {
    Node::div()
        .styles([StyleCmd::FlexCol, StyleCmd::Gap(6.0), StyleCmd::Padding(12.0)])
        .child(Node::text("Welcome!"))
        .child(Link::new("/users/42").child("Open /users/42").build())
        .build()
}

define_route!(IndexRoute, path: "/", is_index: true, node: index_node);
```

Richer screens can use quoin's `component!` macro instead — one source that
compiles for every backend (see `examples/demo-core/src/shell.rs`).

A root layout (`src/routes/__root.rs`) nests children through `<Outlet>`:

```rust
use navi_macros::define_route;
use navi_router::components::{Link, Outlet};
use quoin_ui::{Node, StyleCmd};

fn root_node() -> Node {
    Node::div()
        .styles([StyleCmd::FlexCol])
        .child(Link::new("/").child("Home").build())
        .child(Outlet::new().build())
        .build()
}

define_route!(RootRoute, path: "/", is_layout: true, node: root_node);
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
> Ensure `navi.config.json` is present in your project root (see
> [Configuration](#configuration)).

### 4. Boot the App on Your Framework

The router installation is identical everywhere; only the host bootstrap
differs. The bundled demos show all five:

```rust
// GPUI / Xilem / Floem — quoin's `run_app!` host (one line):
use navi_demo_core::NaviDemo;
use quoin::run_app;
run_app!(NaviDemo);
```

```rust,ignore
// Dioxus desktop
fn app() -> Element { rsx! { NaviDemo {} } }
fn main() { dioxus::LaunchBuilder::desktop().launch(app); }
```

```rust,ignore
// Leptos CSR (wasm32)
#[wasm_bindgen(start)]
pub fn main() {
    leptos::mount::mount_to_body(|| view! { <NaviDemo /> });
}
```

Switching framework = swap the quoin feature + the three‑line host. The route
table, loaders, guards and devtools do not change.

Initialize the router inside your host (or reuse `examples/demo-core/src/host.rs`):

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

## Examples

[`examples/`](https://github.com/elcoosp/gpui-navi/tree/main/examples) ships one shared demo (`demo-core`: routes, loaders,
suspense, guards, search params, 404, devtools) with five thin hosts:

| Example | Framework | Command |
| :--- | :--- | :--- |
| `demo-gpui` | GPUI | `cargo run -p navi-demo-gpui` |
| `demo-dioxus` | Dioxus desktop | `cargo run -p navi-demo-dioxus` |
| `demo-floem` | Floem | `cargo run -p navi-demo-floem` |
| `demo-xilem` | Xilem | `cargo run -p navi-demo-xilem` |
| `demo-leptos` | Leptos CSR | `cargo build -p navi-demo-leptos --target wasm32-unknown-unknown` |

Every host is a 1–3 line bootstrap around the same `NaviDemo` shell — open
them side by side to see how little framework-specific code exists.

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

The `define_route!` macro provides a comprehensive, type‑safe way to
configure routes:

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
    node: my_node,          // framework-agnostic quoin_ui::Node UI
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
| `Link` | Navigation links with built‑in active styling. |
| `Outlet` | Renders the matched child route within a layout. |
| `SuspenseBoundary` | Displays a fallback UI while nested routes load. |
| `Awaited` | Waits for and renders loader data. |
| `ScrollRestoration` | Automatically saves and restores scroll positions. |

## Devtools

> [!TIP]
> Press `Cmd+Shift+D` (or `Ctrl+Shift+D` on Windows/Linux) to open the
> Devtools panel.

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
| Built‑in Devtools | ✅ | ✅ |
| Type‑safe search params | ✅ | ✅ |
| Route context | ✅ | ✅ |
| Deterministic route ranking | ✅ | ✅ |
| Link preloading (render/intent) | ✅ | ✅ |
| Multiple UI frameworks from one route table | ✅ | ❌ |

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

The router is engineered for allocation‑free hot paths — see
[docs/PERFORMANCE.md](https://github.com/elcoosp/gpui-navi/blob/main/docs/PERFORMANCE.md) for the design and
[docs/BENCHMARKS.md](https://github.com/elcoosp/gpui-navi/blob/main/docs/BENCHMARKS.md) for full before/after numbers:

- **Zero‑copy route matching**: stack‑collected parameter bindings, interned
  `Arc<str>` segment names, FxHash static‑child lookup, `HashMap`
  materialized once on success (matching is 14–38% faster across the board).
- **Per‑frame paths allocate nothing**: `use_params!`/`use_search!`/
  `use_loader_data!` read navigation‑time caches; `Link` active checks are
  branch‑and‑byte comparisons (−94%); loader `QueryKey`s are memoized per
  navigation.
- **Bounded memory everywhere**: ring‑buffered event log, capped preload
  registry, O(1) scroll‑eviction, `Arc<RouteTree>` shared between the core
  and the adapter (no deep tree clone at startup).
- **Lock‑free single‑owner history**: no `Mutex` in the navigation path
  (back/forward is −67%).

Quality gates (also enforced in CI):

- `cargo clippy --workspace --all-targets -- -W clippy::pedantic -D warnings`
  — zero warnings, no `#[allow]` attributes in the codebase.
- The four validation integrations (`validator`, `garde`, `validify`,
  `valico`) are mutually exclusive blanket impls, so they are linted and
  tested **per feature** instead of under `--all-features`.
- `clippy.toml` calibrates a few shape thresholds for the devtools UI module
  (large render methods) instead of scattering allow attributes.

The rendering stack is [quoin](https://github.com/elcoosp/quoin) (one
reactive core, many frameworks) and the data stack is
[`rs-query`](https://github.com/elcoosp/rs-query) (query cache with
stale‑time/GC); GPUI is supplied by the matching `gpui-pre` snapshot when
the `gpui` feature is selected.

See [docs/BRAINSTORM.md](https://github.com/elcoosp/gpui-navi/blob/main/docs/BRAINSTORM.md) for the full improvement
brainstorm (performance, DRY/KISS, features) and the remaining follow‑ups
(deep‑links re‑introduction, coverage gate).
