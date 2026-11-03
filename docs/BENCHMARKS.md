# Benchmark results

All numbers are criterion medians on the workspace bench profile (`lto =
"thin"`, `codegen-units = 1`), measured on the same machine in the same
session. "Before" is the saved `before` baseline of the unmodified codebase;
"After" is the optimized codebase. Reproduce:

```sh
cargo bench --bench core --bench adapter -- --save-baseline before \
    --warm-up-time 1 --measurement-time 3 --sample-size 50
# apply the optimization commit
cargo bench --bench core --bench adapter -- --baseline before
```

The route tree used for matching benchmarks holds 130 routes (static,
dynamic, optional, nested and splat mixes) — the shape of matchit's own
benchmarks.

## Route matching (per navigation)

| Benchmark | Before | After | Δ |
| --- | ---: | ---: | ---: |
| match_path/static | 64.8 ns | 41.8 ns | **−35%** |
| match_path/dynamic | 163.2 ns | 131.0 ns | **−19%** |
| match_path/static_beats_dynamic | 60.7 ns | 46.5 ns | **−23%** |
| match_path/optional | 158.2 ns | 132.3 ns | **−16%** |
| match_path/deep_nested | 242.1 ns | 151.3 ns | **−38%** |
| match_path/splat | 204.6 ns | 139.4 ns | **−32%** |
| match_path/root_catchall | 147.4 ns | 125.8 ns | **−14%** |
| match_path/deep_static (11 segs) | 223.8 ns | 173.4 ns | **−23%** |
| match_path/no_match | 172.7 ns | 148.8 ns | **−14%** |

## Location parsing (per navigation)

| Benchmark | Before | After | Δ |
| --- | ---: | ---: | ---: |
| location_new/plain | 34.6 ns | 35.3 ns | ≈ 0 (noise) |
| location_new/query | 323.4 ns | 302.4 ns | **−7%** |
| location_new/query_hash | 446.0 ns | 325.2 ns | **−27%** |
| location_new/coerce (7 values) | 760.5 ns | 632.7 ns | **−17%** |

## Navigation / history (per navigation)

| Benchmark | Before | After | Δ |
| --- | ---: | ---: | ---: |
| navigate/replace_static | 128.1 ns | 108.6 ns | **−15%** |
| navigate/replace_dynamic | 292.1 ns | 190.1 ns | **−35%** |
| history/back_forward | 212.8 ns | 70.3 ns | **−67%** |
| transact/two_replaces | 499.2 ns | 373.1 ns | **−25%** |
| tree/ancestors_depth5 | 163.3 ns | 125.9 ns | **−24%** |
| adapter/navigate_about (full adapter path) | 1.044 µs | 0.853 µs | **−18%** |

## Per-frame render path

| Benchmark | Before | After | Δ |
| --- | ---: | ---: | ---: |
| link/path_is_active_prefix | 36.0 ns | 2.26 ns | **−94%** |
| link/path_is_active_miss | 36.5 ns | 2.08 ns | **−94%** |
| scroll/insert_over_capacity (eviction) | 76.3 ns | 56.9 ns | **−25%** |
| scroll/get_hit | 21.6 ns | 14.8 ns | **−32%** |
| state/current_location_clone | 36.8 ns | 32.5 ns | −14% |
| state/current_match_clone | 23.0 ns | 23.1 ns | ≈ 0 |

Beyond the timed numbers, three per-frame costs were removed entirely by
construction (they no longer allocate at all, so a before/after time is not
meaningful):

- `use_loader_data!` / `get_loader_data` no longer serialize the params map
  and rebuild a `QueryKey` (~8 allocations + one `serde_json::to_string`)
  on every frame — the key is built once per navigation per variant and
  memoized.
- `use_params!` no longer re-serializes the params map every frame — it
  clones a cached `serde_json::Value`.
- `use_search!` no longer clones the full `Location` (pathname + search +
  hash + state) — it clones only the search `Value` from a borrowed read.

## Memory

- `EventLog`: unbounded `Vec<TimedEvent>` → `VecDeque` ring buffer capped at
  1,024 events (constant memory on long sessions, no reallocation after
  warmup).
- Preload dedup registry: unbounded `HashSet<String>` → FxHashSet capped at
  4,096 entries.
- `RouterCore` holds `Arc<RouteTree>`: constructing a router no longer
  deep-clones the whole tree (every node, callback `Arc` and meta map).
- `History` dropped its `Arc<Mutex<...>>` (single-owner by construction).
- `ScrollPositions` eviction is O(1) (`VecDeque::pop_front`) instead of the
  O(n) `Vec::remove(0)`.

## Correctness findings fixed along the way

Writing the coverage suite exposed **two pre-existing deadlocks**: both the
`before_load` NotFound path and the loader `NotFound` outcome path called
`Navigator::push` *inside* a `handle.update` closure — and `parking_lot`
locks are not reentrant. Any app that returned `NotFound` from a guard or a
loader would freeze the UI thread. Both paths now record the state, close
the update, and navigate afterwards (covered by
`test_before_load_redirect_and_not_found` /
`test_loader_factory_redirect_and_not_found`).

## Coverage

`cargo llvm-cov` (including the `navi-test` integration + coverage suites):

- `navi-router-core`: **100.0% line coverage on all 8 source files**.
- `navi-router`: 100% on `event_bus`, `navigator`, `router_provider`,
  `awaited`; `state.rs` 97.5%; `outlet.rs` 99%; `link.rs` 86%;
  `suspense_boundary` 92%; `scroll_restoration` 54%.
- The remaining uncovered lines are, by construction, not reachable without
  a live UI: `Link` event-handler closure bodies (`on_click`,
  `on_mouse_move`), `ScrollRestoration` closures (invoked by quoin's
  renderer), `LoaderState::Loading` (requires a fetch in flight at sampling
  time), and two documented defensive arms. One outlet line (151) is
  verified executed via tracing but attributed to a shared region by
  llvm-cov.
- `navi-devtools` (interactive UI panel) and `navi-macros` internals
  (proc-macro code executing inside rustc) are outside what headless
  coverage can observe.
