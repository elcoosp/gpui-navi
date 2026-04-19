## Analysis of Floem for Universal Router Integration

The Floem codebase reveals a **retained-mode, fine-grained reactive UI framework** built around a few core concepts that directly inform how a router adapter should be designed. This analysis focuses on the features most relevant to implementing `navi-router-floem`.

### 1. Core Reactive Model & State Management

Floem uses its own reactive system (`floem_reactive`) heavily inspired by Leptos.

- **Signals (`RwSignal`, `ReadSignal`, `WriteSignal`):** The primary unit of reactive state. They are `Copy` and `Clone`, making them trivial to pass down the view tree or store in closures. This matches the Leptos "superpower" noted in the brainstorm.

- **Effects (`create_effect`):** Used to run side effects when tracked signals change. Perfect for watching a `RouterCore` location and updating the UI.

- **Memo (`Memo`):** Derived reactive values that only recompute when dependencies change. Ideal for deriving a `RouteMatch` from the current location and route table.

- **`Scope` & Context:** Floem has a scope hierarchy similar to Leptos. Context values can be provided and consumed. The router could provide `RouterCore` as a context value.

**Key Implication for Router:**  
The router state (`RouterCore`) should be stored in a `RwSignal` at the root of the application. The adapter can then provide hooks (or simply expose the signal) to read the current location and navigate.

```rust
// Example of how Floem apps typically share state
let router = RwSignal::new(RouterCore::new(/* ... */));
Context::provide(router); // Provide to children

// In a child view:
let router = Context::get::<RwSignal<RouterCore>>().unwrap();
```

### 2. View Tree & Dynamic Updates

Floem's view tree is **built once** and mutated in place via `ViewId::update_state()`. This is the "Fine-Grained Widget Updates" superpower from the brainstorm.

- **`dyn_container` and `dyn_stack`:** These are Floem's built-in components for swapping out or updating collections of views reactively. They use an effect internally and call `id.update_state()` when the underlying signal changes.

- **`update_state()`:** Any `ViewId` can receive arbitrary state updates. The view's `update()` method can downcast the state and modify its internal data, then request layout/paint as needed.

- **`ParentView` trait (builder pattern):** Floem views can be composed using `.child()`, `.children()`, `.derived_child()`, `.keyed_children()`, etc. This enables declarative, reactive composition.

**Key Implication for Router:**  
An `Outlet` view in Floem should:
1. Hold a `ViewId` for the currently rendered child.
2. Watch the `RouterCore` location signal via an effect.
3. When the route changes, compute the new matched view and call `self.id.update_state(new_view)`.
4. In its `update()` method, replace the child view with the new one.

This avoids rebuilding the entire view tree and is extremely efficient.

```rust
struct Outlet {
    id: ViewId,
    router: RwSignal<RouterCore>,
    current_child: Option<ViewId>,
}

impl View for Outlet {
    fn update(&mut self, cx: &mut UpdateCx, state: Box<dyn Any>) {
        if let Ok(new_view) = state.downcast::<AnyView>() {
            if let Some(old_child) = self.current_child.take() {
                cx.window_state.remove_view(old_child);
            }
            let new_child = new_view.id();
            self.id.set_children([new_view]);
            self.current_child = Some(new_child);
            self.id.request_all();
        }
    }
}
```

### 3. Navigation & History Integration

Floem does not have built-in history or URL management. It relies on `winit` for window events, but there's no platform-agnostic browser history API. The router backend must provide this.

- **Platform Abstraction:** The `navi-backend` crate would define a `HistoryProvider` trait. For desktop apps, it could be a simple in-memory stack; for web (via wasm), it would bind to `window.history`.

- **Navigation Events:** The router adapter should listen to `winit` events (e.g., popstate on web) and update the `RouterCore` signal accordingly. Floem's `action` module provides a way to schedule updates from outside the reactive system.

### 4. Overlays, Modals, and Absolute Positioning

Floem has excellent support for overlays and absolute positioning, which is essential for building modals that are part of the router's nested routes (e.g., `/photos/:id` as a modal).

- **`Overlay` view:** A declarative overlay that automatically reparents its content to the window root, ensuring it paints above all other content with proper z-index. It's perfect for route-based modals.

- **Absolute positioning:** Floem's style system supports `position: absolute` and `fixed`, allowing modal containers to be positioned relative to the viewport or a parent.

- **`NavigationStack`:** Floem already includes a `NavigationStack` view for mobile-style stack navigation (push/pop). While not a full router, it demonstrates that path-based navigation is well-understood in the Floem ecosystem.

**Key Implication for Router:**  
The router can leverage `Overlay` to render modal routes. The `Outlet` for a modal route would wrap its child in an `Overlay`, and the router's `RouteMatch` would indicate that the route should be presented as a modal.

### 5. Window Management

Floem supports multiple windows via `Application::new().window(...)`. Each window has its own `ViewId` root and `WindowHandle`. A universal router should be able to coordinate navigation across multiple windows if desired (e.g., opening a new window for a specific route).

The `navi-router-floem` adapter could provide a `WindowRouter` that manages a separate `RouterCore` per window, or a single global router with window-specific outlets.

### 6. Devtools Integration

Floem has a built-in **inspector** (press F11) that captures the entire view tree, styles, layout, and performance profiles. A router devtools panel could be implemented as a separate window that subscribes to router events (e.g., navigation, loader states) and renders them using Floem's own views. This would be a great showcase of Floem's capabilities.

## Concrete Design for `navi-router-floem`

Based on the analysis, here is a concrete implementation plan for the Floem adapter:

### 1. `RouterBackend` Implementation

```rust
// navi-backend/src/lib.rs (simplified)
pub trait RouterBackend: 'static {
    type Signal<T: 'static>: SignalGet<T> + SignalUpdate<T> + Clone + Copy;
    type Context; // Floem's Scope or similar
    fn request_ui_refresh(cx: &mut Self::Context);
    fn spawn(cx: &mut Self::Context, future: impl Future<Output = ()> + Send + 'static);
}

// navi-router-floem/src/lib.rs
impl RouterBackend for FloemBackend {
    type Signal<T: 'static> = RwSignal<T>;
    type Context = Scope;
    fn request_ui_refresh(cx: &mut Scope) {
        // No explicit refresh needed; signals trigger effects automatically.
        // However, we might need to ensure the effect runs on the next frame.
        // Floem's reactive runtime handles this.
    }
    fn spawn(cx: &mut Scope, future: impl Future<Output = ()> + Send + 'static) {
        // Use tokio or wasm-bindgen-futures; Floem already provides executors.
        floem::spawn(future);
    }
}
```

### 2. `RouterProvider` View

A root view that initializes the router and provides it via context.

```rust
pub struct RouterProvider {
    id: ViewId,
    router: RwSignal<RouterCore<FloemBackend>>,
}

impl RouterProvider {
    pub fn new(routes: RouteTree, initial_location: Location) -> Self {
        let id = ViewId::new();
        let router = RwSignal::new(RouterCore::new(routes, initial_location));
        let outlet = Outlet::new(router);
        id.add_child(outlet.into_any());
        Context::provide(router); // Make available to descendants
        Self { id, router }
    }
}

impl View for RouterProvider {
    fn id(&self) -> ViewId { self.id }
}
```

### 3. `Outlet` View

As described earlier, watches the router location and swaps out the rendered child.

```rust
pub struct Outlet {
    id: ViewId,
    router: RwSignal<RouterCore<FloemBackend>>,
    current_match: Option<RouteMatch>,
    current_child: Option<ViewId>,
}

impl Outlet {
    pub fn new(router: RwSignal<RouterCore<FloemBackend>>) -> Self {
        let id = ViewId::new();
        let outlet = Self { id, router, current_match: None, current_child: None };
        // Set up effect to watch location
        let id_clone = id;
        Effect::new(move |_| {
            let router = router.get();
            let new_match = router.match_location(router.location());
            id_clone.update_state((new_match, router));
        });
        outlet
    }
}

impl View for Outlet {
    fn update(&mut self, cx: &mut UpdateCx, state: Box<dyn Any>) {
        if let Ok((new_match, router)) = state.downcast::<(RouteMatch, RouterCore<FloemBackend>)>() {
            // Only rebuild if the match actually changed
            if Some(&new_match) != self.current_match.as_ref() {
                let view = router.render_match(&new_match); // Returns AnyView
                self.current_match = Some(new_match.clone());
                // Replace child
                if let Some(old) = self.current_child.take() {
                    cx.window_state.remove_view(old);
                }
                let new_id = view.id();
                self.id.set_children([view]);
                self.current_child = Some(new_id);
                self.id.request_all();
            }
        }
    }
}
```

### 4. Navigation Primitives

Provide ergonomic hooks for navigation:

```rust
pub fn use_navigate() -> impl Fn(Location) + Clone {
    let router = Context::get::<RwSignal<RouterCore<FloemBackend>>>().unwrap();
    move |to| router.update(|r| r.navigate(to))
}

pub fn use_location() -> Location {
    let router = Context::get::<RwSignal<RouterCore<FloemBackend>>>().unwrap();
    router.with(|r| r.location().clone())
}

pub fn use_params<T: Params>() -> T {
    let router = Context::get::<RwSignal<RouterCore<FloemBackend>>>().unwrap();
    router.with(|r| r.current_match().params().clone())
}
```

### 5. Modal Routes

Leverage Floem's `Overlay`:

```rust
impl RouterCore<FloemBackend> {
    pub fn render_match(&self, m: &RouteMatch) -> AnyView {
        let view = m.route().component()(m.params().clone());
        if m.route().modal() {
            Overlay::new(view).into_any()
        } else {
            view
        }
    }
}
```

### 6. Link Component

A simple button that navigates:

```rust
pub fn link(to: impl Into<Location>, child: impl IntoView) -> impl IntoView {
    let to = to.into();
    Button::new(child).action(move || {
        let navigate = use_navigate();
        navigate(to.clone());
    })
}
```

### 7. Devtools Panel

Since Floem has a built-in inspector, we can create a `RouterDevtools` view that displays the current route tree, active matches, loader states, and navigation history. This could be toggled via a keyboard shortcut (e.g., Ctrl+Shift+R) and rendered as an overlay.

## Conclusion

Floem's architecture is exceptionally well-suited for a universal router adapter. Its retained-mode widget tree with fine-grained updates, powerful reactive primitives, and built-in overlays make implementing a performant and feature-rich router straightforward. The design outlined above aligns perfectly with the "Fine-Grained Widget Updates" section of the brainstorm and demonstrates how Floem's unique strengths can be celebrated in a unified routing ecosystem.
