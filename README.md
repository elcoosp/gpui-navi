# Navi

**Navi** is an experimental, type‑safe router for [GPUI](https://www.gpui.rs/) applications, inspired by [TanStack Router](https://tanstack.com/router). It provides a solid foundation for declarative routing with file‑based code generation, path matching, and a layered context system. **Please note: Navi is under active development—many components are placeholders and not yet functional.**

---

## 🚧 Current Status

| Component               | Status                                                                                         |
| ----------------------- | ---------------------------------------------------------------------------------------------- |
| Route definition macros | ✅ Functional – `define_route!` and `define_router!` generate valid route trees.                 |
| Path matching           | ✅ Functional – supports static, dynamic (`$id`), optional, and splat segments.                  |
| File‑based codegen      | ✅ Functional – scans `src/routes` and outputs `route_tree.gen.rs`.                             |
| Context tree            | ✅ Functional – layered context for dependency injection.                                       |
| In‑memory history       | ✅ Functional – stack‑based navigation history (stub, no browser integration).                  |
| Router components       | 🚧 Stubbed – `<Outlet>`, `<Link>`, `<RouterProvider>` are empty `div`s; no rendering logic.     |
| Programmatic navigation | 🚧 Stubbed – `Navigator` methods are no‑ops.                                                    |
| Loaders / data caching  | 🚧 Planned – `rs-query` integration exists but loaders are not executed.                        |
| Suspense boundaries     | 🚧 Stubbed – configuration exists, but no fallback rendering.                                   |
| Devtools                | 🚧 Stubbed – structs and event log present, but UI is unimplemented.                            |
| Scroll restoration      | 🚧 Stubbed – `HashMap` for positions, but never used.                                           |
| Validation framework    | 🚧 Trait defined – no concrete implementations or integrations.                                 |

**What this means for you:** Navi currently provides a robust route *matching* engine and code generation tooling, but it does **not** yet render routes or handle navigation out of the box. You can use the matching logic and context system in your own rendering setup, but the high‑level components are not ready.

---

## Project Structure

Navi is a Cargo workspace containing several crates:

| Crate                      | Description                                                                                      |
| -------------------------- | ------------------------------------------------------------------------------------------------ |
| `navi-core`                | Core primitives: layered `ContextTree` and `SuspenseState` enum.                                  |
| `navi-router`              | Route tree, pattern matching, history stub, and stubbed components.                               |
| `navi-macros`              | Procedural macros for defining routes and hooks (`define_route!`, `use_params!`, etc.).           |
| `navi-codegen`             | File‑based route discovery and code generator.                                                    |
| `navi-devtools`            | Devtools panel structs (UI not implemented).                                                      |
| `example-app`              | Demonstrates route definition and matching (no actual rendering).                                 |
| `stubs/history-navigation` | Simple in‑memory history implementation.                                                          |

---

## Getting Started (Current Reality)

### Adding Navi to Your Project

```toml
[dependencies]
navi-router = { path = "path/to/navi-router" }
navi-macros = { path = "path/to/navi-macros" }
navi-core = { path = "path/to/navi-core" }
```

### Defining Routes

Use the `define_route!` macro to create a route type with a pattern:

```rust
use navi_macros::define_route;

define_route!(
    UserRoute,
    path: "/users/$id",
    params: UserParams,      // optional
    search: UserSearch,      // optional
);
```

The macro generates a `UserRoute` struct that implements `navi_router::RouteDef` and provides a `build_node()` method for adding it to a route tree.

### Building a Route Tree

Manually construct a `RouteTree` and add nodes:

```rust
use navi_router::{RouteTree, Location};

let mut tree = RouteTree::new();
tree.add_route(UserRoute::build_node());

// Match a path
if let Some((params, node)) = tree.match_path("/users/42") {
    println!("Matched route: {} with params {:?}", node.id, params);
}
```

### File‑Based Code Generation (Optional)

If you prefer to generate the route tree from your file system:

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

3. Place route files in `src/routes` following the naming conventions (e.g., `users.rs`, `$id.rs`). The build script will generate a `build_route_tree()` function.

### Using the Context Tree (for Manual Integration)

Navi’s context system allows you to provide and consume values across nested scopes:

```rust
use navi_core::context;

let window_id = gpui::WindowId(0);
context::init_window(window_id);
context::provide(window_id, "some value");
let value: Option<String> = context::consume(window_id);
```

This is the foundation for dependency injection in routes, but the router does **not** automatically populate route params or loader data into the context.

---

## What’s Missing (And How You Can Help)

Navi is an open‑source project looking for contributors. The core architecture is solid, but the following areas need implementation:

- **Component rendering** – Make `<Outlet>`, `<Link>`, `<RouterProvider>` actually render GPUI elements and respond to navigation.
- **Navigation execution** – Connect `Navigator` to `RouterState` so `push`/`replace`/`back` modify history and update the UI.
- **Loader integration** – Execute loader functions, cache results with `rs-query`, and provide data to components.
- **Suspense** – Implement fallback rendering logic in `SuspenseBoundary`.
- **Devtools UI** – Build a GPUI‑based panel that displays routes, cache, and timeline.
- **Browser history** – Replace the in‑memory stub with actual `window.history` API calls (WASM‑compatible).
- **Validation adapters** – Implement `ValidateSearch` for popular crates (`validator`, `garde`).

If you’re interested in contributing, please open an issue or pull request. We especially welcome help with the rendering pipeline and loader execution.

---

## Example App

The `example-app` crate demonstrates how to define routes and test matching, but **it does not render a UI**. You can run it to see the route tree and matching in action:

```bash
cargo run -p example-app
```

Output:
```
Navi Example App - Router initialized successfully!
Registered routes:
  __root__ -> /
  index -> /
  users_index -> /users
  user_detail -> /users/$id
  settings -> /settings
Matched / -> __root__ ({})
Matched /users -> users_index ({})
Matched /users/42 -> user_detail ({"id": "42"})
Matched /settings -> settings ({})
No match for /unknown
```

---

## License

Navi is licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

**Navi** – Building a type‑safe router for GPUI, one piece at a time.
