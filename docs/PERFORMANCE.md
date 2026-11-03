# Performance Engineering

This document is the design record of the performance pass over the navi
workspace: what was measured, what SOTA techniques were applied, and what the
benchmarks say. Benchmarks live in `navi-router-core/benches/` and
`navi-router/benches/` (criterion). Reproduce with `cargo bench -p
navi-router-core -p navi-router`.

## 1. Where the time goes

Profiling by inspection of the render/navigation loops identified two classes
of hot code:

**Per-frame (runs on every repaint, for every mounted hook / link / outlet):**

| Site | Cost |
| --- | --- |
| `use_params!` | `current_match.clone()` (params `HashMap` clone) + `serde_json::to_value` re-serialization of the params map |
| `use_search!` | full `Location` clone (pathname + deep `serde_json::Value` search + hash + state) + a second `Value` clone |
| `use_loader_data!` → `get_loader_data` | rebuilds the `QueryKey` every call: `serde_json::to_string(params)` + up to 8 `String`/`format!` allocations (`QueryKey::new/with`), then a cache lookup |
| `Link::build` | `current_location()` full clone + `format!("{href}/")` allocation in `path_is_active` |
| `Outlet::build` | `ancestors()` allocates a `String` per ancestor hop |
| `EventLog::push/get` | unbounded `Vec` growth (memory) and full-`Vec` clone per `get()` |
| `ScrollPositions::insert` | `Vec::remove(0)` eviction is O(n) |

**Per-navigation (runs on every push/replace/back/forward):**

| Site | Cost |
| --- | --- |
| `RouteTrie::match_path` | `Vec<&str>` segment allocation, a fresh (empty) `HashMap` per match attempt, `name.clone() + seg.to_string()` per dynamic hop — all repeated during backtracking |
| `RouterState::new_with_options` | `(*route_tree).clone()` deep-clones the entire `RouteTree` (every node, callback `Arc` and meta map) into `RouterCore` |
| `History` | `Arc<Mutex<BrowserHistory>>` lock/unlock on every operation although `RouterCore` owns the history exclusively; `current()` clones the whole `Location` |
| `Location::new` | query parsing allocates per pair regardless of shape |
| `RouterCore::navigate_with` | `params.clone()` even when the route has no loader |

## 2. SOTA techniques applied

The techniques below are the ones the current state of the art converges on
(matchit's zero-copy radix router, the Rust Performance Book's hashing and
allocation guidance, Zed/GPUI's "no allocations in the render path" practice,
and ring-buffer bounded-memory patterns):

1. **Zero-copy matching (matchit-style).** The matcher walks path segments on
   the stack (`SmallVec<[&str; 8]>` — heap only beyond 8 segments), collects
   `(name, value)` bindings in a stack `SmallVec`, and materializes the
   public `HashMap<String, String>` **once, on final success**. Failed
   branches no longer allocate anything. Static-child lookups move from
   `BTreeMap<String, _>` to a sorted small-vec with binary search (fan-out is
   tiny; linear/binary scan beats tree hops and SipHash).
2. **Interned segment/param names.** Trie nodes share param names as
   `Arc<str>` (cloned from the parsed pattern once), so binding extraction is
   a refcount bump instead of a `String` clone.
3. **`FxHash` for internal string-keyed maps** (Rust Performance Book:
   FxHash wins for short string keys when HashDoS is not a threat — a router
   tree is author-controlled, not attacker-controlled). Applied to the route
   node map, children map, the outlet registry, the preload dedup set, and
   blocker/loader/context maps. Public API types stay `std::HashMap`.
4. **Shared `Arc<RouteTree>`.** `RouterCore` holds `Arc<RouteTree>` instead of
   an owned deep clone: constructing a router no longer duplicates the whole
   tree, and `RouteTree: Clone` remains for callers who want value semantics.
5. **Lock-free single-owner history.** `History` drops its
   `Arc<Mutex<...>>` (it is owned exclusively by `RouterCore`; every method
   already took `&mut self`) and adds `current_ref()` so traversal and
   adapters can read the current `Location` by reference.
6. **Per-navigation caches in the adapter.** `RouterState` computes, exactly
   once per committed navigation: the params JSON `Value` (for
   `use_params!`) and the loader `QueryKey` (for `get_loader_data`/`get_loader_state`).
   Per-frame hooks now clone a cached `Value`/`QueryKey` instead of
   re-serializing.
7. **Borrowed state accessors.** `current_pathname()`, `current_hash()`,
   `search_value()` let `Link`, `ScrollRestoration` and the hooks avoid the
   full `Location` clone.
8. **Bounded ring buffers.** `EventLog` becomes a `VecDeque` with a capacity
   (default 1024) that evicts the oldest event: constant memory on long
   sessions, no reallocation after warmup, and `get()` clones a bounded
   snapshot. `ScrollPositions` eviction becomes O(1) `pop_front`.
9. **Allocation-free string predicates.** `path_is_active` drops its
   `format!` and compares bytes; `canonical_search_key` builds its key in one
   buffer; `parse_query_string` dispatches on the first value byte before
   attempting numeric coercion and reserves map capacity.
10. **Bounded preload dedup registry.** The global `PRELOAD_STARTED` set is
    capped (4096 keys, wholesale reset beyond) and hashed with FxHash: no
    unbounded growth in long-running apps.
11. **Release-profile tuning.** Workspace `release` profile: `lto = "thin"`,
    `codegen-units = 1`; `bench` profile keeps symbols for profiling.

## 3. Semantics preserved

All existing tests pass unchanged except where a signature intentionally
changed (`RouterCore::new` now takes `Arc<RouteTree>`; `History` lost its
`Arc<Mutex>` internals while keeping its API). Matching priority (static >
dynamic > optional > splat), query coercion, scroll-key canonicalization,
event ordering and blocker behavior are covered by the pre-existing suites
plus new tests written first for every optimization.

## 4. Results

See `docs/BENCHMARKS.md` for the full criterion before/after tables. The
summary is regenerated by running:

```sh
cargo bench -p navi-router-core -p navi-router -- --save-baseline before
# ... apply changes ...
cargo bench -p navi-router-core -p navi-router -- --baseline before
```
