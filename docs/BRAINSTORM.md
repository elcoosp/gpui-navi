# Navi Router — Improvement Brainstorm & Changelog of Ideas

This document records the full brainstorm performed on the codebase (performance,
modularity, DRY/KISS, and complementary features) and what was implemented as a
result. Each item is marked **[done]** or **[future]**.

---

## 1. Correctness bugs found while auditing (all fixed)

1. **[done] Nondeterministic route matching.** The matching trie iterated an
   unordered `HashMap` of children, so two routes that could match the same
   path (e.g. `/users/new` vs `/users/$id`) raced on hash-iteration order —
   the same navigation could resolve differently run to run. The trie now
   partitions children into `BTreeMap` static children (probed first, O(log n)),
   insertion-ordered dynamic params, optional params, and a single splat.
2. **[done] `get_loader_data` silently returned `None` for plain data.** The
   loader macro stores `Arc<T>` holding the payload, but the accessor downcast
   to `Arc<R::LoaderData>` — which only worked when users *additionally*
   wrapped their data in `Arc` (the example app did exactly that to hide the
   bug). The accessor now accepts both plain and `Arc`-wrapped payloads.
3. **[done] `RouterEvent` was never emitted.** The event system existed,
   devtools rendered a timeline — but no navigation ever fired an event.
   `BeforeNavigate`, `BeforeLoad`, and `Resolved` are now dispatched through
   both subscriber callbacks and the global event log (devtools timeline works).
4. **[done] Missing `garde` validation integration.** The feature flag and
   README existed, but no `ValidateSearch` impl was ever written for `garde`.
5. **[done] Query-string parsing did not percent-decode.** `?name=Na%20vi`
   kept the encoded form. Parsing now uses `url::form_urlencoded`, coerces
   scalars (int/float/bool), and collects repeated keys into JSON arrays
   (`?tag=a&tag=b` → `["a","b"]`).
6. **[done] `Location::to_url` dropped non-string search values.** Numbers and
   booleans are now serialized; hashes without a leading `#` are handled.
7. **[done] Broken `nexum` path dependencies.** They pointed outside the repo
   (`../../nexum/...`) so the workspace could not even resolve. Both crates
   are now proper git dependencies.
8. **[done] Link active-state matching.** `starts_with` matched `/users-admin`
   as active for `/users`; the documented `include_hash`/`include_search`
   options were a TODO. Active matching is now segment-boundary aware and
   honors both options.
9. **[done] `Link` preload was dead config.** `preload`/`preload_delay`
   fields were stored and never read. `PreloadType::Render/Viewport` preload
   during first render; `Intent` arms a hover handler with a configurable
   delay; all de-duplicated via a registry so renders never spam the cache.
10. **[done] Double route matching per navigation.** `RouterState` matched the
    path, then `RouterCore::navigate` matched it again. The core now takes a
    precomputed match (`navigate_with`), halving trie work per navigation.
11. **[done] Not-found recursion crash.** Navigating to an unknown path with
    no `/404` route pushed `/404` again → another `NotFound` effect → infinite
    recursion through nested `update_global` calls (SIGABRT). Guarded by a
    recursion flag; the effect handler now navigates directly instead of
    re-entering `RouterState::update` (which gpui forbids).
12. **[done] Split `not_found_data`.** `RouterCore` and `RouterState` each
    kept their own `not_found_data`; the core's `None` overwrote the state's
    value on every not-found navigation. They are now synced.
13. **[done] Codegen layout detection.** `infer_layout` resolved sibling
    directories against the process CWD (broken for absolute configs) and
    ignored the explicit `is_layout: true` in route files — so
    `dashboard/mod.rs` children were mis-parented to the tree root in the
    shipped generated file. Fixed both; numeric route files (`404.rs`) also
    generated invalid module identifiers and are now prefixed.
14. **[done] `define_route!` parser.** Path expressions like
    `stale_time: std::time::Duration::from_secs(30)` failed to parse (they
    were mistaken for types); unknown fields were silently ignored instead of
    erroring; the generated `build_router` from `define_router!` called
    `RouterState::new` with a stale signature and could never compile.
15. **[done] `xattr` build break.** Vendored a minimal patched `xattr 0.2.4`
    (see README) because `async-tar` → `xattr 0.2.3` no longer compiles on
    Linux against current libc.

## 2. Performance

1. **[done] Deterministic ordered trie** (also correctness, see above) with
   static-first probing — no more `HashMap` iteration per segment.
2. **[done] Idempotent codegen output.** `write_route_tree` now skips the write
   when content is unchanged; build.rs no longer dirties the crate on every
   build (spurious rebuild killer).
3. **[done] Cached scanner regexes.** The codegen scanner compiled three regexes
   per scanned file; they are now compiled once (`OnceLock`).
4. **[done] Single-match navigation** (`navigate_with`).
5. **[done] `Arc`-based `RouteNode` sharing.** The trie stores every route
   behind an `Arc<RouteNode>` and `match_path` hands it out by clone of the
   `Arc` — matching no longer deep-clones callbacks, metadata or pattern
   strings, and `current_match` copies are pointer bumps.
6. **[done] History traversal re-matches the route tree.** `back`/`forward`/
   `go` (and the same ops inside transactions) re-resolve `current_match`,
   so the UI renders the route that is actually on screen. Previously a
   back-swipe kept rendering the stale route.

## 2. Dependency stack policy (decision)

All crates.io dependencies were bumped to their latest majors (thiserror 2,
syn 3, serde_qs removed, schemars 1.2, validify 2, darling 0.24, convert_case
0.12, parking_lot 0.13, garde 0.23, validator 0.21, …). Mid-2026 the
`gpui-component` upstream renamed and split itself (`gpui-component` →
`gpui-kit` + `gpui-component` 0.8, `gpui-component-assets` → `gpui-kit-assets`)
and relocated most shared infrastructure into the unstyled `gpui-base` crate.
**The migration is done** (see §3.5): the workspace now sits on the released
post-restructure line — `gpui-component` 0.8.1 / `gpui-kit` 0.8.1 from
crates.io, GPUI supplied by the matching `gpui-pre =0.3.10` snapshot of
recent zed main declared under the `gpui` alias, and `rs-query` pinned to
`ee416b8` with its optional `devtools` feature still off (Navi ships its own
devtools UI; the pin's used API surface is gpui-free, so the crate's
internal zed-git gpui dependency stays a graph leaf and cannot clash with
the snapshot).

## 3. Modularity

1. **[done] gpui-free core.** `navi-router-core` never depended on gpui; it is
   now the only place validation integrations live (see DRY), with
   `navi-router` features simply forwarding (`validator =
   ["navi-router-core/validator"]`), so downstream crates enable features in
   one place and the blanket impls actually turn on.
2. **[done] Lean dependency graph.** `gpui-component` was declared by
   `navi-router` and `navi-core` without being used — removed (big win for
   compile times of both crates and everything downstream).
3. **[done] Workspace facade.** `navi-router` re-exports the full core API
   (`AnyData`, `LoaderState`, `PreloadType`, `Awaited`, `ScrollRestoration`,
   …) so users depend on `navi-router` + `navi-macros` only.
4. **[future] Backend story.** `docs/backends/` already sketches Dioxus/Floem/
   Xilem ports. With the core truly UI-free, a second backend only needs a
   `*-router` adapter crate mirroring `navi-router`.

5. **[done] gpui-kit migration.** Migrated `navi-devtools`, `example-app`
    and the `rs-query` pin to the post-restructure `gpui-component` 0.8 /
    `gpui-kit` stack (GPUI via the `gpui-pre =0.3.10` snapshot of recent zed
    main). Most symbols survived the split between `gpui-base` and
    `gpui-component`; the concrete deltas that had to be ported: `Root`'s
    dialog/sheet/notification layers are mounted by the Root itself (the
    `Root::render_*_layer` calls are gone, `gpui_kit::open_window` replaces
    the manual `Root::new` dance), `gpui::Component` disappeared (route
    components now go through `IntoElement::into_any_element`), the
    `dropdown_menu` closure receives `&mut Context<PopupMenu>` instead of
    `&mut App`, gpui's `test-support` must be requested explicitly now that
    rs-query's zed-git gpui is a separate package, and the `xattr` vendored
    patch became unnecessary (`async-tar` left the graph with
    `gpui-pre-reqwest-client`). The devtools table/timeline code ported with
    unchanged `TableDelegate`/`Column`/`Tab`/`TabBar`/`VirtualList` shapes
    and was visually verified against the example app.

## 4. DRY

1. **[done] Dead duplicate files deleted.** `navi-router/src/{history,
   location,validation,redirect,blocker}.rs` were stale copies of
   `navi-router-core` files, undeclared in `lib.rs` — deleted.
2. **[done] One `AnyData`.** Defined in both `navi-router-core` and
   `navi-router`; now a single definition (core) re-exported by the adapter.
   Gained `AnyData::new` / `downcast_ref` / `get_cloned` helpers.
3. **[done] Loader cache-key extraction.** `get_loader_data` and
   `get_loader_state` duplicated ~30 lines of `QueryKey` construction; both now
   share `loader_query_key` / `loader_key_for_current`.
4. **[done] Validation roundtrip helper.** All four validation integrations
   duplicated the JSON serialize/deserialize error plumbing; now one
   `deserialize_from_raw` + `to_query_via_json` + `single_error` trio.
5. **[done] Macro crate de-duplicated deps.** `darling` and `convert_case`
   were declared but unused — removed.

## 5. KISS

1. **[done] `Segment` simplified.** `prefix`/`suffix` fields existed but were
   never parsed into nor read. `Segment` is now
   `Static(String) | Dynamic{name} | Optional{name} | Splat`.
2. **[done] `once_cell` replaced with `std::sync::LazyLock`** (Rust ≥ 1.80) in
   all three crates that used it; the dependency is gone.
3. **[done] Dead code removed:** codegen stub generators (with their stacked
   duplicate `#[allow(dead_code)]` attributes), unused `thiserror`/`anyhow`/
   `serde_qs`/`seahash`/`url`/`futures` deps across crates.
4. **[done] std-first codegen keywords list.** Single const slice instead of a
   giant inline `match`.

## 6. Ambitious complementary features

1. **[done] Route-tree structural validation** — `RouteTree::validate()`
   detects unknown parents, parent cycles, and duplicate patterns (layout/page
   overlap stays legal), with a typed `RouteTreeError`.
2. **[done] Working event timeline** (see bug #3) — devtools timeline now
   receives `BeforeNavigate → BeforeLoad → Resolved` in real time.
3. **[done] Link preloading strategies** (render / intent / viewport-fallback)
   with delays and de-duplication.
4. **[done] Correct active-state matching incl. search/hash comparison.**
5. **[done] Query arrays + decoding** — typed search params can now model
   multi-value filters without hand-parsing.
6. **[done] Config discovery** — `NaviConfig::discover` walks up from the cwd
   to find `navi.config.json`, so the CLI and build scripts work in
   subdirectories.
7. **[done] `RouterState::root_view` / `window_handle` accessors** (devtools
   previously reached into a `pub` field from another crate's copy-paste).
8. **[done] Nested search params via `serde_qs`.** The `nested-search`
   feature (forwarded by `navi-router`) adds `parse_query_nested` /
   `query_to_string`: typed structs with nested objects and indexed arrays
   (`filter[status]=open&tags[0]=rust`). `serde_html_form` was evaluated
   first but only supports flat maps + repeated keys, so `serde_qs` (1.1)
   is the dependency.
9. **[done] Navigation transactions**: `NavOp` (`Push`/`Replace`/`Back`/
   `Forward`/`Go`) batches are applied by `RouterCore::transact` in one
   synchronous step with exactly one `NotifyListeners` effect (one
   re-render per batch); unmatched pushes produce `NotFound` and leave
   history untouched. `RouterState::transact` wraps it with the event pair
   and falls back to sequential `navigate` calls when blockers are armed;
   `Navigator::transact` is the ergonomic entry point.
10. **[done] Scroll restoration v2**: entries are keyed by pathname **plus a
    canonicalized search string** (`?b=2&a=1` and `?a=1&b=2` share one
    entry), the store is capacity-bounded (default 128, oldest evicted,
    `ScrollRestoration::set_capacity`), the router-level
    `scroll_restoration: false` flag is finally honored (it used to be
    dead config), and a navigation with `reset_scroll: Some(true)` skips
    restoring exactly once.
11. **[done] `Link` external hrefs.** `http(s):`, `mailto:` and `tel:` links
    are handed to the OS (`cx.open_url`) instead of being matched against
    the route tree; active styles and preloads are skipped for them.
12. **[done] `Link::reset_scroll` is live.** The builder flag flows through
    `NavigateOptions.reset_scroll` into the scroll-restoration suppression
    described above.
13. **[future] Optional WASM/no-op backend of `history-navigation` so the
    core can be fuzzed without a display server. Partially covered: the
    core is already probed by a dependency-free, model-based randomized
    navigation suite (see §7), which runs headless everywhere.

## 7. Testing & CI/CD

1. **[done] Test suite expanded** from 9 tests to a full matrix: unit tests in
   every crate (route matching determinism, pattern ranking, location
   round-trips, blockers, redirects, validation per feature flag, codegen
   fixtures on disk, history listener semantics, context trees), plus
   gpui-powered `TestAppContext` integration tests for `RouterState`
   (navigation, blockers, loader registration, query keys, meta, event log).
2. **[done] Model-based randomized navigation suite.** A seeded xorshift RNG
   drives hundreds of random `Push`/`Replace`/`Back`/`Forward`/`Go`
   sequences through `RouterCore` and compares every step against a plain
   `Vec` + cursor model (history length, current path, back/forward
   possibility and the `current_match` ⇔ location invariant). No external
   fuzzing dependency, fully deterministic, headless. It caught the
   transaction/traversal re-match bug on its first run.
3. **[done] CI** (`.github/workflows/ci.yml`): rustfmt, clippy pedantic with
   `-D warnings` and no allows, workspace tests on Linux + macOS, coverage via
   cargo-llvm-cov, example build, docs with broken-link denial.
4. **[done] Release pipeline** (`.github/workflows/release.yml`): verify →
   git-cliff changelog → cross-platform `navi-codegen` artifacts → GitHub
   Release with generated notes.
5. **[done] Changelog bot** (`.github/workflows/changelog.yml` +
   `cliff.toml`): keeps `CHANGELOG.md` updated on every push to main.
6. **[done] `justfile` recipes** for every check used in CI.
7. **[done] Lint policy.** Clippy pedantic with `-D warnings` and no
   `#[allow]` attributes; the policy is codified in the workspace
   `[workspace.lints]` table (pedantic warn + `unsafe_code` deny, inherited
   by every crate through `[lints] workspace = true`); the validation
   features are linted/tested per feature (they are mutually exclusive
   blanket impls by design) and `clippy.toml` calibrates shape thresholds
   for the devtools UI module.
8. **[future] Coverage gate.** CI runs `cargo-llvm-cov` and publishes the
   report; a hard 100%-line-coverage gate is the follow-up once the gpui-kit
   migration settles (UI render paths are best covered with gpui's
   `TestAppContext`, as done in `navi-test`.
