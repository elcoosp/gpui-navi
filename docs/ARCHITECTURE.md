# Navi Router Architecture

## Core (`navi-router-core`)

Pure routing logic with zero UI framework dependencies:
- `RouterCore` — state machine returning `NavigationEffect` enum
- `RouteTree` — O(log n) radix trie for path matching
- `History` — browser-style history stack
- `Blocker` — navigation guard
- `Location`, `Redirect`, `NotFound`, validation traits

## Adapters

Adapters wrap `RouterCore` and interpret `NavigationEffect`s for a specific UI framework:
- `navi-router` — GPUI adapter (uses `cx.spawn`, `QueryClient`)
- Future: Dioxus, egui, etc.

## Writing a New Adapter

1. Instantiate `RouterCore` with a `RouteTree`
2. On navigation, call `core.navigate(loc)` and handle effects
3. For `SpawnLoader`, use your framework's async runtime
4. For `NotifyListeners`, refresh your UI
