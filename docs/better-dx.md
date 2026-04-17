# Navi — Comprehensive DX Improvement Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Elevate Navi to a world‑class, delightful router for GPUI by fixing file‑based routing (especially `$` in filenames), replacing fragile regex parsing with robust AST analysis, integrating `darling` for beautiful macro errors, and polishing every rough edge in hooks, components, and conventions.

**Architecture:** The existing architecture (route pattern matching, history stack, context tree, loader/query integration, and devtools) is solid. This plan focuses on the **developer experience** layer: `navi-codegen` (scanner and generator) and `navi-macros`. We will split file‑stem processing into two distinct functions (`module_ident` vs `url_segment`), replace regex file content scanning with `syn` AST walking, and use `darling` to provide span‑accurate error messages with “did you mean?” suggestions.

**Tech Stack:** Rust, GPUI, `syn`, `darling`, `quote`, `proc-macro2`.

---

## Chunk 1: Fix `$` in Filenames – Correct Module Identifiers and URL Patterns

**Files:**
- Modify: `navi-codegen/src/scanner.rs`

### Task 1.1: Split `sanitize_module_ident` into Two Functions

- [ ] **Step 1: Define `file_stem_to_module_ident` (Rust module name)**

```rust
/// Converts a filesystem name (like "$id", "$", "about") to a valid Rust module identifier.
fn file_stem_to_module_ident(stem: &str) -> String {
    let s = stem.replace('-', "_").replace('.', "_");
    let ident = if s == "$" {
        "splat".to_string()
    } else if s.starts_with('$') {
        format!("param_{}", &s[1..])
    } else {
        s
    };
    escape_rust_keyword(ident)
}

fn escape_rust_keyword(s: String) -> String {
    match s.as_str() {
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern"
        | "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match"
        | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self"
        | "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe"
        | "use" | "where" | "while" | "async" | "await" | "dyn" | "union"
        => format!("r#{}", s),
        _ => s,
    }
}
```

- [ ] **Step 2: Define `file_stem_to_url_segment` (route pattern segment)**

```rust
/// Returns the URL segment contributed by a file or directory name,
/// or `None` if it contributes nothing (root, index, layout, group).
fn file_stem_to_url_segment(name: &str) -> Option<String> {
    if matches!(name, "__root" | "index") {
        return None;
    }
    if name.starts_with('_') {
        return None; // layout files
    }
    if name.starts_with('(') && name.ends_with(')') {
        return None; // pathless groups
    }
    Some(name.to_string())
}
```

- [ ] **Step 3: Update `build_module_path` to use `file_stem_to_module_ident` exclusively**

```rust
fn build_module_path(relative_path: &Path, is_mod_rs: bool) -> String {
    let mut components: Vec<String> = relative_path
        .parent()
        .into_iter()
        .flat_map(|p| p.iter())
        .map(|c| file_stem_to_module_ident(c.to_str().unwrap()))
        .collect();

    if !is_mod_rs {
        let file_stem = relative_path.file_stem().unwrap().to_str().unwrap();
        components.push(file_stem_to_module_ident(file_stem));
    }
    components.join("::")
}
```

- [ ] **Step 4: Update `file_name_to_pattern` to use `file_stem_to_url_segment`**

```rust
fn file_name_to_pattern(file_name: &str, relative_path: &Path) -> String {
    let mut segments = Vec::new();
    for component in relative_path.parent().into_iter().flat_map(|p| p.iter()) {
        if let Some(comp) = component.to_str() {
            if comp.starts_with('(') && comp.ends_with(')') { continue; }
            if comp.starts_with('-') { continue; }
            if let Some(seg) = file_stem_to_url_segment(comp) {
                segments.push(seg);
            }
        }
    }
    if let Some(seg) = file_stem_to_url_segment(file_name) {
        segments.push(seg);
    }
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}
```

- [ ] **Step 5: Run `cargo check -p navi-codegen` to verify no regressions**

- [ ] **Step 6: Commit**

```bash
git add navi-codegen/src/scanner.rs
git commit -m "fix(navi-codegen): separate module ident from url segment for $ files"
```

---

## Chunk 2: Replace Regex File Content Parsing with `syn` AST Analysis

**Files:**
- Modify: `navi-codegen/src/scanner.rs`
- Modify: `navi-codegen/Cargo.toml`

### Task 2.1: Remove `regex` Dependency, Ensure `syn` Features

- [ ] **Step 1: Update `navi-codegen/Cargo.toml`**

```toml
[dependencies]
# Remove: regex = "1.10"
syn = { version = "2.0", features = ["full", "visit", "parsing"] }
# keep other deps...
```

- [ ] **Step 2: Implement `extract_route_type_from_ast` using `syn`**

```rust
use syn::{File, Item, Macro, parse_file};

fn extract_route_type_from_ast(content: &str) -> Option<String> {
    let file = parse_file(content).ok()?;
    visit_items_for_define_route(&file.items)
}

fn visit_items_for_define_route(items: &[Item]) -> Option<String> {
    for item in items {
        match item {
            Item::Macro(m) if macro_is_define_route(&m.mac) => {
                return first_ident_in_macro(&m.mac);
            }
            Item::Mod(m) => {
                if let Some((_, content)) = &m.content {
                    if let Some(name) = visit_items_for_define_route(content) {
                        return Some(name);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn macro_is_define_route(mac: &Macro) -> bool {
    mac.path.segments.last().map(|s| s.ident == "define_route").unwrap_or(false)
}

fn first_ident_in_macro(mac: &Macro) -> Option<String> {
    use syn::parse::Parser;
    use syn::punctuated::Punctuated;
    use syn::{Ident, Token};
    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
    let idents = parser.parse2(mac.tokens.clone()).ok()?;
    idents.first().map(|i| i.to_string())
}
```

- [ ] **Step 3: Implement `extract_cfg_feature_from_ast`**

```rust
fn extract_cfg_feature_from_ast(content: &str) -> Option<String> {
    let file = parse_file(content).ok()?;
    for item in &file.items {
        if let Item::Macro(m) = item {
            if macro_is_define_route(&m.mac) {
                for attr in &m.attrs {
                    if attr.path().is_ident("cfg") {
                        if let Ok(meta) = attr.parse_args::<syn::Meta>() {
                            if let syn::Meta::NameValue(nv) = meta {
                                if nv.path.is_ident("feature") {
                                    if let syn::Expr::Lit(lit) = nv.value {
                                        if let syn::Lit::Str(s) = lit.lit {
                                            return Some(s.value());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
```

- [ ] **Step 4: Replace calls to regex‑based extractors in `parse_route_file`**

```rust
fn parse_route_file(/* ... */) -> Result<RouteInfo> {
    // ...
    let route_type_name = extract_route_type_from_ast(content)
        .unwrap_or_else(|| infer_route_type_name(file_stem, relative_path));
    let cfg_feature = extract_cfg_feature_from_ast(content);
    // ...
}
```

- [ ] **Step 5: Verify with `cargo check -p navi-codegen`**

- [ ] **Step 6: Commit**

```bash
git add navi-codegen/src/scanner.rs navi-codegen/Cargo.toml
git commit -m "refactor(navi-codegen): replace regex with syn AST parsing"
```

---

## Chunk 3: Fix `assign_parents` – Pure In‑Memory Hierarchy Inference

**Files:**
- Modify: `navi-codegen/src/scanner.rs`

### Task 3.1: Rewrite `assign_parents` Without File I/O

- [ ] **Step 1: Remove the existing `assign_parents` implementation**

- [ ] **Step 2: Implement new `assign_parents`**

```rust
fn assign_parents(routes: &mut Vec<RouteInfo>) {
    use std::collections::HashMap;
    use std::path::PathBuf;

    // Map from directory path to the route ID of the layout that owns it
    let mut dir_to_layout: HashMap<PathBuf, String> = HashMap::new();

    for route in routes.iter() {
        if route.is_layout || route.is_root {
            let dir = if route.relative_path.file_name().unwrap() == "mod.rs" {
                route.relative_path.parent().unwrap().to_path_buf()
            } else {
                route.relative_path.with_extension("")
            };
            dir_to_layout.insert(dir, route.route_id.clone());
        }
    }

    let root_id = routes.iter()
        .find(|r| r.is_root)
        .map(|r| r.route_id.clone());

    for route in routes.iter_mut() {
        if route.is_root {
            continue;
        }
        let mut search = route.relative_path.parent().unwrap_or(Path::new("")).to_path_buf();
        while let Some(layout_id) = dir_to_layout.get(&search) {
            route.parent = Some(layout_id.clone());
            break;
        }
        if route.parent.is_none() {
            route.parent = root_id.clone();
        }
    }
}
```

- [ ] **Step 3: Run `cargo check -p navi-codegen`**

- [ ] **Step 4: Commit**

```bash
git add navi-codegen/src/scanner.rs
git commit -m "fix(navi-codegen): assign parents purely in-memory"
```

---

## Chunk 4: Codegen Output Overhaul – Emit Module Tree and `register_routes`

**Files:**
- Modify: `navi-codegen/src/generator.rs`

### Task 4.1: Emit `#[path]` Module Declarations in Generated File

- [ ] **Step 1: In `generator.rs`, build a list of unique module paths**

```rust
use std::collections::BTreeSet;

let mut module_decls = BTreeSet::new();
for route in &routes {
    let mod_path = &route.module_path;
    let file_path = route.relative_path.to_str().unwrap();
    module_decls.insert(format!("#[path = \"routes/{}\"] pub mod {};", file_path, mod_path.replace("::", "_")));
}
```

- [ ] **Step 2: Generate `register_routes` function**

```rust
let mut register_calls = String::new();
for route in &routes {
    let mod_ident = route.module_path.replace("::", "_");
    let route_type = &route.route_type_name;
    if let Some(feature) = &route.cfg_feature {
        register_calls.push_str(&format!(
            "#[cfg(feature = \"{}\")] _routes::{}::{}::register(cx);\n",
            feature, mod_ident, route_type
        ));
    } else {
        register_calls.push_str(&format!(
            "_routes::{}::{}::register(cx);\n",
            mod_ident, route_type
        ));
    }
}
```

- [ ] **Step 3: Update the template string**

```rust
let output = format!(
    r#"// AUTO-GENERATED by navi-codegen - DO NOT EDIT

pub mod _routes {{
{module_decls}
}}

/// Build the route tree with all discovered routes.
pub fn build_route_tree() -> navi_router::RouteTree {{
    let mut tree = navi_router::RouteTree::new();
{route_nodes}
    tree
}}

/// Register all route components and loaders.
pub fn register_routes(cx: &mut gpui::App) {{
{register_calls}
}}
"#,
    module_decls = module_decls.into_iter().collect::<Vec<_>>().join("\n"),
    route_nodes = route_nodes,
    register_calls = register_calls,
);
```

- [ ] **Step 4: Test with example-app**

```bash
cargo run -p navi-codegen
cargo check -p example-app
```

- [ ] **Step 5: Update `example-app/src/main.rs` to use `register_routes`**

```rust
mod route_tree {
    include!("route_tree.gen.rs");
}
// inside main, after RouterProvider::new:
route_tree::register_routes(cx);
```

- [ ] **Step 6: Remove manual registration from `example-app/src/main.rs`**

- [ ] **Step 7: Commit**

```bash
git add navi-codegen/src/generator.rs example-app/src/main.rs
git commit -m "feat(navi-codegen): emit module tree and register_routes"
```

---

## Chunk 5: `navi-macros` – Integrate `darling` for Beautiful Errors

**Files:**
- Modify: `navi-macros/Cargo.toml`
- Modify: `navi-macros/src/route.rs`

### Task 5.1: Add `darling` Dependency

- [ ] **Step 1: Update `navi-macros/Cargo.toml`**

```toml
[dependencies]
darling = "0.20"
```

### Task 5.2: Rewrite `RouteDefArgs` with `darling`

- [ ] **Step 2: Replace the hand‑rolled parser in `route.rs`**

```rust
use darling::{FromMeta, ast::NestedMeta, Error as DarlingError};
use syn::{ExprClosure, Ident, LitBool, LitStr, Type};

#[derive(Debug, FromMeta)]
struct RouteDefArgs {
    path: LitStr,
    #[darling(default)]
    params: Option<syn::Path>,
    #[darling(default)]
    search: Option<syn::Path>,
    #[darling(default)]
    data: Option<syn::Path>,
    #[darling(default)]
    loader: Option<syn::Expr>,
    #[darling(default)]
    component: Option<syn::Path>,
    #[darling(default)]
    stale_time: Option<syn::Expr>,
    #[darling(default)]
    gc_time: Option<syn::Expr>,
    #[darling(default)]
    preload_stale_time: Option<syn::Expr>,
    #[darling(default)]
    is_layout: Option<LitBool>,
    #[darling(default)]
    is_index: Option<LitBool>,
    #[darling(default)]
    parent: Option<LitStr>,
}

pub fn define_route(input: TokenStream) -> TokenStream {
    let input2: proc_macro2::TokenStream = input.into();

    // Split off the route name (first identifier)
    let mut iter = input2.clone().into_iter();
    let name_tt = match iter.next() {
        Some(t) => t,
        None => return DarlingError::custom("define_route! requires a route name").write_errors().into(),
    };
    let name: Ident = match syn::parse2(proc_macro2::TokenStream::from(name_tt)) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error().into(),
    };

    let remaining: proc_macro2::TokenStream = iter.collect();

    let attr_args = match NestedMeta::parse_meta_list(remaining) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let args = match RouteDefArgs::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return e.write_errors().into(),
    };

    // Validation
    if args.loader.is_some() && args.data.is_none() {
        return quote::quote_spanned! { name.span() =>
            compile_error!("define_route!: `loader` requires `data` to be specified.");
        }.into();
    }

    // Generate code...
    expand_define_route(name, args).into()
}
```

- [ ] **Step 3: Implement `expand_define_route` using the same code generation logic as before, but using `args` fields**

- [ ] **Step 4: Test with a deliberate typo to see the improved error message**

```rust
// example-app/src/routes/broken.rs
define_route!(
    BrokenRoute,
    path: "/broken",
    compnent: BrokenPage, // typo
);
```

Expected output points to `compnent` and suggests `component`.

- [ ] **Step 5: Commit**

```bash
git add navi-macros/Cargo.toml navi-macros/src/route.rs
git commit -m "feat(navi-macros): integrate darling for span-accurate errors"
```

---

## Chunk 6: Hook Macros – Fixes and Additions

**Files:**
- Modify: `navi-macros/src/hooks.rs`
- Modify: `navi-macros/src/lib.rs`
- Modify: `navi-router/src/state.rs`

### Task 6.1: Fix `use_can_go_back!`

- [ ] **Step 1: Replace the stub implementation**

```rust
pub fn use_can_go_back(_input: TokenStream) -> TokenStream {
    quote! {
        ::navi_router::Navigator::can_go_back(cx)
    }.into()
}
```

### Task 6.2: Remove Loader Trigger from `use_loader_data!`

- [ ] **Step 2: Simplify to read‑only**

```rust
pub fn use_loader_data(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    quote! {
        {
            let state = ::navi_router::RouterState::global(cx);
            state.get_loader_data::<#route_ty>()
        }
    }.into()
}
```

### Task 6.3: Add `use_match!` Macro

- [ ] **Step 3: Implement `use_match!` in `hooks.rs`**

```rust
pub fn use_match(_input: TokenStream) -> TokenStream {
    quote! {
        {
            let state = ::navi_router::RouterState::global(cx);
            state.current_match.clone()
                .map(|(params, node)| (node.id.clone(), params))
                .unwrap_or_default()
        }
    }.into()
}
```

- [ ] **Step 4: Re‑export from `lib.rs`**

```rust
pub use hooks::use_match;
```

### Task 6.4: Add `use_loader_state!` Macro

- [ ] **Step 5: In `state.rs`, add `LoaderState` enum**

```rust
#[derive(Clone, Debug)]
pub enum LoaderState {
    Idle,
    Loading,
    Ready,
    Error(String),
}

impl RouterState {
    pub fn get_loader_state<R: RouteDef>(&self) -> LoaderState {
        // Check query client for the specific key status
        // ...
    }
}
```

- [ ] **Step 6: Implement macro in `hooks.rs`**

```rust
pub fn use_loader_state(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    quote! {
        {
            let state = ::navi_router::RouterState::global(cx);
            state.get_loader_state::<#route_ty>()
        }
    }.into()
}
```

- [ ] **Step 7: Verify build**

```bash
cargo check -p navi-macros -p navi-router
```

- [ ] **Step 8: Commit**

```bash
git add navi-macros/src/hooks.rs navi-macros/src/lib.rs navi-router/src/state.rs
git commit -m "fix(navi-macros): correct use_can_go_back, remove loader side-effect, add use_match, use_loader_state"
```

---

## Chunk 7: `Link` Component – Fix Active Styling

**Files:**
- Modify: `navi-router/src/components/link.rs`

### Task 7.1: Replace String‑Based Active Class with Styling Closures

- [ ] **Step 1: Update `Link` struct to hold style closures**

```rust
pub struct Link {
    href: String,
    active_style: Option<Box<dyn Fn(Div) -> Div>>,
    inactive_style: Option<Box<dyn Fn(Div) -> Div>>,
    exact: bool,
    // ... other fields
}
```

- [ ] **Step 2: Add builder methods**

```rust
impl Link {
    pub fn active_style(mut self, f: impl Fn(Div) -> Div + 'static) -> Self {
        self.active_style = Some(Box::new(f));
        self
    }

    pub fn inactive_style(mut self, f: impl Fn(Div) -> Div + 'static) -> Self {
        self.inactive_style = Some(Box::new(f));
        self
    }

    pub fn exact(mut self) -> Self {
        self.exact = true;
        self
    }
}
```

- [ ] **Step 3: Apply styles in `RenderOnce`**

```rust
impl RenderOnce for Link {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_active = RouterState::try_global(cx)
            .map(|s| {
                let current = &s.current_location().pathname;
                if self.exact {
                    current == &self.href
                } else {
                    current.starts_with(&self.href)
                }
            })
            .unwrap_or(false);

        let mut el = div()
            .cursor_pointer()
            .on_mouse_up(MouseButton::Left, { /* navigate */ })
            .children(self.children);

        if is_active {
            if let Some(f) = self.active_style {
                el = f(el);
            } else {
                el = el.font_weight(FontWeight::BOLD);
            }
        } else if let Some(f) = self.inactive_style {
            el = f(el);
        }
        el
    }
}
```

- [ ] **Step 4: Update `example-app` usage to the new API**

```rust
Link::new("/users")
    .active_style(|el| el.bg(rgb(0x2563eb)).text_color(white()))
    .child("Users")
```

- [ ] **Step 5: Verify rendering in example-app**

```bash
cargo run -p example-app
```

- [ ] **Step 6: Commit**

```bash
git add navi-router/src/components/link.rs example-app/src/**/*.rs
git commit -m "fix(link): use styling closures for active/inactive states"
```

---

## Chunk 8: Route Ranking – Add Depth Scaling

**Files:**
- Modify: `navi-router/src/route_tree.rs`

### Task 8.1: Update `compute_rank` to Include Depth

- [ ] **Step 1: Replace the current rank computation**

```rust
fn compute_rank(segments: &[Segment]) -> usize {
    let depth = segments.len();
    let static_count = segments.iter().filter(|s| s.is_static()).count();
    let dynamic_count = segments.iter().filter(|s| s.is_dynamic()).count();
    let optional_count = segments.iter().filter(|s| s.is_optional()).count();
    let has_splat = segments.iter().any(|s| s.is_splat());

    let mut rank = depth * 10_000;
    rank += static_count * 100;
    rank += dynamic_count * 10;
    rank += optional_count * 5;
    if has_splat {
        rank = rank.saturating_sub(5_000);
    }
    rank
}
```

- [ ] **Step 2: Test with example routes**

Verify that `/users/settings` (2 static) ranks higher than `/users/$id` (1 static + 1 dynamic).

- [ ] **Step 3: Commit**

```bash
git add navi-router/src/route_tree.rs
git commit -m "fix(route_tree): incorporate depth in route ranking"
```

---

## Chunk 9: Suspense Boundary and Scroll Restoration Completion

**Files:**
- Modify: `navi-router/src/components/catch_boundary.rs` → rename to `suspense_boundary.rs`
- Modify: `navi-router/src/components/scroll_restoration.rs`
- Modify: `navi-router/src/components/mod.rs`

### Task 9.1: Implement `SuspenseBoundary`

- [ ] **Step 1: Create `suspense_boundary.rs` (or repurpose `catch_boundary.rs`)**

```rust
use gpui::*;
use crate::RouterState;

pub struct SuspenseBoundary {
    fallback: Box<dyn Fn() -> AnyElement>,
}

impl SuspenseBoundary {
    pub fn new(fallback: impl Fn() -> AnyElement + 'static) -> Self {
        Self { fallback: Box::new(fallback) }
    }
}

impl RenderOnce for SuspenseBoundary {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_loading = RouterState::try_global(cx)
            .map(|s| s.has_pending_loader())
            .unwrap_or(false);
        if is_loading {
            (self.fallback)()
        } else {
            Outlet::new().into_any_element()
        }
    }
}
```

### Task 9.2: Add `has_pending_loader` to `RouterState`

- [ ] **Step 2: In `state.rs`**

```rust
pub fn has_pending_loader(&self) -> bool {
    self.query_client.is_fetching()
}
```

### Task 9.3: Basic Scroll Restoration

- [ ] **Step 3: In `scroll_restoration.rs`, add save/restore logic**

```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;

static SCROLL_POSITIONS: Lazy<Mutex<HashMap<String, f32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub struct ScrollRestoration;

impl ScrollRestoration {
    pub fn save(path: &str, y: f32) {
        SCROLL_POSITIONS.lock().unwrap().insert(path.to_string(), y);
    }

    pub fn get(path: &str) -> Option<f32> {
        SCROLL_POSITIONS.lock().unwrap().get(path).copied()
    }
}
```

- [ ] **Step 4: Wire save on navigation, restore in `Outlet`**

- [ ] **Step 5: Commit**

```bash
git add navi-router/src/components/
git commit -m "feat: implement SuspenseBoundary and scroll restoration"
```

---

## Chunk 10: `define_router!` – Emit `register_routes`

**Files:**
- Modify: `navi-macros/src/router.rs`

### Task 10.1: Add `register_routes` Function Generation

- [ ] **Step 1: Update the macro expansion**

```rust
let register_calls = input.routes.iter().map(|route| {
    quote! { #route::register(cx); }
});

let expanded = quote! {
    pub enum Route { #( #route_variants, )* }

    pub fn build_route_tree() -> navi_router::RouteTree { /* ... */ }

    pub fn register_routes(cx: &mut gpui::App) {
        #( #register_calls )*
    }
};
```

- [ ] **Step 2: Verify with a test route**

- [ ] **Step 3: Commit**

```bash
git add navi-macros/src/router.rs
git commit -m "feat(define_router): generate register_routes function"
```

---

## Final Steps

- Run `cargo test --all` to ensure no regressions.
- Update `README.md` with new usage instructions (file‑based routing works with `$`, simplified registration).
- Merge and celebrate.

**Plan complete.** Ready for execution.
