# Navi Router

A powerful file‑based router for [GPUI](https://github.com/zed-industries/zed) with loaders, suspense, devtools, and full TanStack Router feature parity.

## Features

- **File‑based routing** – Define routes via the filesystem.
- **Nested layouts** – Hierarchical layouts with `<Outlet>`.
- **Loaders** – Data fetching with caching, stale‑time, and GC.
- **Suspense boundaries** – Granular loading states with `Awaited`.
- **`beforeLoad` hooks** – Route guards with redirect/notFound.
- **Navigation blockers** – Async/sync blockers for unsaved changes.
- **Scroll restoration** – Automatic scroll position memory.
- **Devtools** – Inspect routes, cache, timeline, and state.
- **Validation** – Integrates with `validator`, `garde`, `validify`, `valico`.
- **Codegen** – Automatic route tree generation from `src/routes`.
- **Type‑safe hooks** – `use_params!`, `use_search!`, `use_loader_data!`, `use_navigate!`, etc.

## Quick Start

Add to your `Cargo.toml`:

```toml
navi-router = { path = "navi-router" }
navi-macros = { path = "navi-macros" }
```

Create a root layout:

```rust
// src/routes/__root.rs
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

Add an index route:

```rust
// src/routes/index.rs
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

Configure code generation in `build.rs`:

```rust
fn main() {
    let config = navi_codegen::NaviConfig::from_file("navi.config.json")
        .expect("Failed to read config");
    navi_codegen::generator::write_route_tree(&config).unwrap();
}
```

In `main.rs`:

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

`navi.config.json`:

```json
{
  "routes_directory": "./src/routes",
  "generated_route_tree": "./src/route_tree.gen.rs",
  "route_token": "route",
  "index_token": "index",
  "route_file_ignore_prefix": "-"
}
```

## Route Definition Macro

```rust
define_route!(
    MyRoute,
    path: "/users/$id",
    params: UserParams,
    search: UserSearch,
    data: User,
    loader: |params, executor| async move { ... },
    before_load: |ctx| async move { BeforeLoadResult::Ok },
    on_enter: |loc| log::info!("entered"),
    on_leave: |loc| log::info!("left"),
    stale_time: Duration::from_secs(60),
    gc_time: Duration::from_secs(300),
    meta: { ... },
    component: MyComponent,
);
```

## Hooks

- `use_params!(MyRoute)`
- `use_search!(MyRoute)`
- `use_loader_data!(MyRoute)`
- `use_navigate!()`
- `use_matched_route!(MyRoute)`
- `use_route_context!(MyRoute)`

## Components

- `Link` – Navigation links with active styling.
- `Outlet` – Renders nested routes.
- `SuspenseBoundary` – Shows fallback while loading.
- `Awaited` – Waits for loader data.
- `ScrollRestoration` – Saves/restores scroll position.

## Devtools

Press `Cmd+Shift+D` to open. Tabs:
- **Routes** – Tree view, test navigation.
- **Cache** – Inspect/invalidate rs‑query cache.
- **Timeline** – Event log with search/filter/export.
- **State** – Router state, blockers, meta.

## Feature Comparison

| Feature | Navi Router | TanStack Router |
|---------|-------------|-----------------|
| File‑based routing | ✅ | ✅ |
| Nested layouts | ✅ | ✅ |
| Loaders | ✅ | ✅ |
| `beforeLoad` | ✅ | ✅ |
| Async blockers | ✅ | ✅ |
| Scroll restoration | ✅ | ✅ |
| Devtools | ✅ | ✅ |
| Type‑safe search params | ✅ | ✅ |
| Route context | ✅ | ✅ |

## License

MIT
