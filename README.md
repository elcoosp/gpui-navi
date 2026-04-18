# Navi

<p align="center">
  <a href="https://raw.githubusercontent.com/elcoosp/gpui-navi/main/assets/logos/horizontal.svg">
      <img src="https://raw.githubusercontent.com/elcoosp/gpui-navi/main/assets/logos/horizontal.svg" alt="rs-query horizontal logo" width="500">
  </a>
</p>

**Navi** is an experimental, type‑safe router for [GPUI](https://www.gpui.rs/) applications, inspired by [TanStack Router](https://tanstack.com/router). It provides a solid foundation for declarative routing with file‑based code generation, path matching, and a layered context system.

---

## 🚧 Current Status

| Component               | Status                                                                                         |
| ----------------------- | ---------------------------------------------------------------------------------------------- |
| Route definition macros | ✅ Functional – `define_route!` and `define_router!` generate valid route trees.                |
| Path matching           | ✅ Functional – supports static, dynamic (`$id`), optional, and splat segments.                 |
| File‑based codegen      | ✅ Functional – scans `src/routes` and outputs `route_tree.gen.rs`.                             |
| Context tree            | ✅ Functional – layered context for dependency injection.                                      |
| In‑memory history       | ✅ Functional – stack‑based navigation history (desktop‑friendly, no browser integration).     |
| Router components       | ✅ Functional – `<Outlet>`, `<Link>`, `<RouterProvider>` render GPUI elements and handle navigation. |
| Programmatic navigation | ✅ Functional – `Navigator::push`, `replace`, `back`, `forward` work with `RouterState`.        |
| Loaders / data caching  | ✅ Functional – async loaders with caching (basic `HashMap` cache).                             |
| Suspense boundaries     | 🚧 Planned – configuration exists, but no fallback rendering component.                        |
| Devtools                | ✅ Functional – event timeline, badge coloring, and a GPUI panel (UI implemented).             |
| Scroll restoration      | ❌ Missing – placeholder component exists but does not save/restore positions.                 |
| Validation framework    | ✅ Functional – `ValidateSearch` trait with integrations for `validator`, `garde`, `validify`, `valico`. |

**What this means for you:** Navi is a fully functional router for GPUI desktop applications. You can define routes, navigate, load data, and render nested UIs. The core is production‑ready; the remaining work focuses on advanced features like suspense, scroll restoration, and cache invalidation.

---

## Project Structure

Navi is a Cargo workspace containing several crates:

| Crate                      | Description                                                                                      |
| -------------------------- | ------------------------------------------------------------------------------------------------ |
| `navi-core`                | Core primitives: layered `ContextTree` and `SuspenseState` enum.                                  |
| `navi-router`              | Route tree, pattern matching, history, loaders, components (`Outlet`, `Link`, etc.), and state.   |
| `navi-macros`              | Procedural macros for defining routes and hooks (`define_route!`, `use_params!`, etc.).           |
| `navi-codegen`             | File‑based route discovery and code generator.                                                    |
| `navi-devtools`            | Devtools panel with event timeline, cache inspector, and navigation tools.                        |
| `example-app`              | Full GPUI application demonstrating routing, loaders, validation, and blockers.                   |
| `stubs/history-navigation` | Simple in‑memory history implementation (suitable for desktop).                                   |

---

## Getting Started

### Adding Navi to Your Project

```toml
[dependencies]
navi-router = { path = "path/to/navi-router" }
navi-macros = { path = "path/to/navi-macros" }
navi-core = { path = "path/to/navi-core" }
```

### Defining a Route with Loader

```rust
use navi_macros::define_route;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct UserParams {
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct UserData {
    pub name: String,
}

define_route!(
    UserRoute,
    path: "/users/$id",
    params: UserParams,
    data: UserData,
    loader: |params: UserParams, executor: gpui::BackgroundExecutor| async move {
        // Simulate async fetch
        executor.timer(std::time::Duration::from_millis(500)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            std::sync::Arc::new(UserData { name: format!("User {}", params.id) })
        )
    },
    component: UserPage,
);

#[derive(Clone, IntoElement)]
struct UserPage;
impl RenderOnce for UserPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = use_loader_data!(UserRoute);
        match data {
            Some(data) => div().child(format!("Hello, {}", data.name)),
            None => div().child("Loading..."),
        }
    }
}
```

### Building the Route Tree and Starting the Router

```rust
use navi_router::{Location, RouteTree, RouterProvider};

let mut tree = RouteTree::new();
tree.add_route(UserRoute::build_node());

// In your GPUI app initialization:
let provider = RouterProvider::new(
    window_id,
    window_handle,
    Location::new("/"),
    tree,
    cx,
);
```

### File‑Based Code Generation (Optional)

1. Create a `navi.config.json`:

```json
{
  "routes_directory": "./src/routes",
  "generated_route_tree": "./src/route_tree.gen.rs"
}
```

2. Add a build script (`build.rs`):

```rust
fn main() {
    navi_codegen::write_route_tree(&navi_codegen::NaviConfig::default()).unwrap();
}
```

3. Place route files in `src/routes` (e.g., `users/$id.rs`). The build script generates a `build_route_tree()` function that you can use to create the `RouteTree`.

---

## What’s Missing (And How You Can Help)

Navi is actively developed. The following features are not yet implemented (or are partially implemented) – contributions welcome:

- **Suspense boundaries** – Show a fallback UI while loaders are pending.
- **Scroll restoration** – Preserve scroll position on back/forward navigation.
- **Loader cache invalidation** – Add TTL (`stale_time`, `gc_time`) and manual invalidation API.
- **Parallel data loading** – Load data for nested routes concurrently.
- **Preloading / prefetching** – Implement the `preload` prop on `<Link>`.
- **Search param middleware** – Integrate `RetainSearchParams` and `StripSearchParams` into navigation.

**Not planned (out of scope for this router):**
- Browser `window.history` integration (Navi targets desktop GPUI, not web).
- Server‑side rendering or static site generation.

If you’re interested in contributing, please open an issue or pull request. We especially welcome help with suspense boundaries and cache invalidation.

---

## Example App

The `example-app` crate demonstrates a fully functional GPUI application with routing, loaders, search param validation, navigation blockers, and a devtools panel. Run it with:

```bash
cargo run -p example-app
```

The app includes:
- Nested layouts (`/users` layout with `<Outlet />`)
- Dynamic routes (`/users/$id` with async loader)
- Search param validation and sorting (`/users?sort=asc`)
- Navigation blocker demo (`/settings` page)
- Devtools panel with event timeline

---

## License

Navi is licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

**Navi** – Building a type‑safe router for GPUI, one piece at a time.
