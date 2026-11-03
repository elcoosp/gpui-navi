use crate::location::Location;
use crate::redirect::{NotFound, Redirect};
use crate::state_types::AnyData;
use futures::future::BoxFuture;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Clone)]
pub struct RouteContextArgs {
    pub parent_context: Option<serde_json::Value>,
    pub params: HashMap<String, String>,
    pub loader_data: Option<AnyData>,
}

pub struct BeforeLoadContext {
    pub params: HashMap<String, String>,
    pub search: serde_json::Value,
    pub location: Location,
}

pub enum BeforeLoadResult {
    Ok,
    Redirect(Redirect),
    NotFound(NotFound),
}

pub type BeforeLoadFn =
    Arc<dyn Fn(BeforeLoadContext) -> BoxFuture<'static, BeforeLoadResult> + Send + Sync>;

/// Callback fired when a route is entered.
pub type OnRouteEventFn = Arc<dyn Fn(&Location) + Send + Sync>;
/// Derives extra loader-cache dependencies from the current search params.
pub type LoaderDepsFn = Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Segment {
    Static(String),
    Dynamic { name: String },
    Optional { name: String },
    Splat,
}

impl Segment {
    #[must_use]
    pub fn param_name(&self) -> Option<&str> {
        match self {
            Segment::Dynamic { name } | Segment::Optional { name } => Some(name),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_static(&self) -> bool {
        matches!(self, Segment::Static(_))
    }

    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Segment::Dynamic { .. })
    }

    #[must_use]
    pub fn is_optional(&self) -> bool {
        matches!(self, Segment::Optional { .. })
    }

    #[must_use]
    pub fn is_splat(&self) -> bool {
        matches!(self, Segment::Splat)
    }
}

#[derive(Clone, Debug)]
pub struct RoutePattern {
    pub raw: String,
    pub segments: Vec<Segment>,
    pub rank: usize,
}

impl RoutePattern {
    #[must_use]
    pub fn parse(pattern: &str) -> Self {
        let raw = pattern.to_string();
        let segments = Self::parse_segments(pattern);
        let rank = Self::compute_rank(&segments);
        Self {
            raw,
            segments,
            rank,
        }
    }

    fn parse_segments(pattern: &str) -> Vec<Segment> {
        pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|part| {
                if part.starts_with('[') && part.ends_with(']') && part.len() > 1 {
                    // "[name]" is an escaped static segment.
                    Segment::Static(part[1..part.len() - 1].to_string())
                } else if let Some(rest) = part.strip_prefix("{-$") {
                    // "{-$name}" is an optional parameter segment.
                    let name = rest.strip_suffix('}').unwrap_or(rest);
                    Segment::Optional {
                        name: name.to_string(),
                    }
                } else if let Some(rest) = part.strip_prefix("{$") {
                    // "{$name}" is a dynamic parameter segment (same as "$name").
                    let name = rest.strip_suffix('}').unwrap_or(rest);
                    Segment::Dynamic {
                        name: name.to_string(),
                    }
                } else if part == "$" {
                    Segment::Splat
                } else if let Some(name) = part.strip_prefix('$') {
                    Segment::Dynamic {
                        name: name.to_string(),
                    }
                } else {
                    Segment::Static(part.to_string())
                }
            })
            .collect()
    }

    fn compute_rank(segments: &[Segment]) -> usize {
        let depth = segments.len();
        let static_count = segments.iter().filter(|s| s.is_static()).count();
        let dynamic_count = segments.iter().filter(|s| s.is_dynamic()).count();
        let optional_count = segments.iter().filter(|s| s.is_optional()).count();
        let has_splat = segments.iter().any(Segment::is_splat);
        let mut rank = depth * 10_000;
        rank += static_count * 100;
        rank += dynamic_count * 10;
        rank += optional_count * 5;
        if has_splat {
            rank = rank.saturating_sub(5_000);
        }
        rank
    }
}

#[derive(Clone)]
pub struct RouteNode {
    pub id: String,
    pub pattern: RoutePattern,
    pub parent: Option<String>,
    pub is_layout: bool,
    pub is_index: bool,
    pub has_loader: bool,
    pub loader_stale_time: Option<std::time::Duration>,
    pub loader_gc_time: Option<std::time::Duration>,
    pub preload_stale_time: Option<std::time::Duration>,
    pub before_load: Option<BeforeLoadFn>,
    pub on_enter: Option<OnRouteEventFn>,
    pub on_leave: Option<OnRouteEventFn>,
    pub loader_deps: Option<LoaderDepsFn>,
    pub context_fn: Option<Arc<dyn Fn(RouteContextArgs) -> serde_json::Value + Send + Sync>>,
    pub meta: HashMap<String, serde_json::Value>,
}

impl std::fmt::Debug for RouteNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteNode")
            .field("id", &self.id)
            .field("pattern", &self.pattern.raw)
            .field("is_layout", &self.is_layout)
            .field("is_index", &self.is_index)
            .field("has_loader", &self.has_loader)
            .field("has_before_load", &self.before_load.is_some())
            .finish_non_exhaustive()
    }
}

// ---- Radix trie ----
//
// Children are partitioned by segment kind so that matching is deterministic:
// exact static children are probed first (FxHashMap — exact-key lookup only,
// so iteration order is never observed and the hashed keys are
// author-controlled, not attacker-controlled), then dynamic parameters in
// insertion order, then optional parameters, and finally the splat. This
// guarantees that for `/users/new` the static route always wins over
// `/users/$id`, regardless of registration order.
//
// Segment and parameter names are interned as `Arc<str>` (shared with the
// parsed pattern) so matching only bumps refcounts, never copies strings.
#[derive(Clone, Debug, Default)]
struct RadixNode {
    static_children: FxHashMap<Arc<str>, RadixNode>,
    param_children: Vec<(Arc<str>, RadixNode)>,
    optional_children: Vec<(Arc<str>, RadixNode)>,
    splat_child: Option<Box<RadixNode>>,
    route: Option<Arc<RouteNode>>,
}

impl RadixNode {
    fn or_default_param(&mut self, name: &str) -> &mut RadixNode {
        if let Some(index) = self
            .param_children
            .iter()
            .position(|(n, _)| n.as_ref() == name)
        {
            return &mut self.param_children[index].1;
        }
        let interned: Arc<str> = Arc::from(name);
        self.param_children.push((interned, RadixNode::default()));
        let last = self.param_children.len() - 1;
        &mut self.param_children[last].1
    }

    fn or_default_optional(&mut self, name: &str) -> &mut RadixNode {
        if let Some(index) = self
            .optional_children
            .iter()
            .position(|(n, _)| n.as_ref() == name)
        {
            return &mut self.optional_children[index].1;
        }
        let interned: Arc<str> = Arc::from(name);
        self.optional_children
            .push((interned, RadixNode::default()));
        let last = self.optional_children.len() - 1;
        &mut self.optional_children[last].1
    }

    fn static_child(&self, name: &str) -> Option<&RadixNode> {
        self.static_children.get(name)
    }

    fn insert_static_child(&mut self, name: &str) -> &mut RadixNode {
        let interned: Arc<str> = Arc::from(name);
        self.static_children.entry(interned).or_default()
    }
}

#[derive(Clone, Debug, Default)]
struct RouteTrie {
    root: RadixNode,
}

/// A successful match: the extracted parameters plus the matched node.
/// The node is `Arc`-shared with the tree, so matching never deep-clones
/// route callbacks or metadata.
pub type MatchResult = Option<(HashMap<String, String>, Arc<RouteNode>)>;

impl RouteTrie {
    fn insert(&mut self, node: Arc<RouteNode>) {
        let mut current = &mut self.root;
        for seg in &node.pattern.segments {
            current = match seg {
                Segment::Static(s) => current.insert_static_child(s),
                Segment::Dynamic { name } => current.or_default_param(name),
                Segment::Optional { name } => current.or_default_optional(name),
                Segment::Splat => current.splat_child.get_or_insert_with(Box::default),
            };
        }
        current.route = Some(node);
    }

    fn match_path(&self, path: &str) -> MatchResult {
        // Zero-copy: path segments live on the stack (heap only past 8);
        // parameter bindings are collected as `(name, value)` borrows and the
        // public `HashMap` is materialized exactly once, on final success.
        let mut segments: SmallVec<[&str; 8]> = SmallVec::new();
        for part in path.split('/').filter(|s| !s.is_empty()) {
            segments.push(part);
        }
        let mut bindings: SmallVec<[(&str, BindingValue<'_, '_>); 8]> = SmallVec::new();
        match match_segments(&segments, &self.root, &mut bindings) {
            None => None,
            Some(route) => {
                let mut params = HashMap::with_capacity(bindings.len());
                for (name, value) in bindings {
                    let value = match value {
                        BindingValue::Slice(s) => s.to_string(),
                        BindingValue::Splat(parts) => parts.join("/"),
                    };
                    params.insert(name.to_string(), value);
                }
                Some((params, Arc::clone(route)))
            }
        }
    }
}

/// A parameter binding value discovered during matching. Splat captures span
/// several segments, so they are joined only when the match succeeds.
#[derive(Clone, Copy)]
enum BindingValue<'s, 'p> {
    Slice(&'p str),
    Splat(&'s [&'p str]),
}

/// Depth-first matcher over the trie. Free function because it only needs the
/// current node (recursion), not the trie itself. Tentative bindings are
/// pushed onto `bindings` and rolled back when a branch fails, so failed
/// branches never allocate and successful branches keep exactly the bindings
/// that led to the match.
fn match_segments<'a, 's, 'p>(
    segments: &'s [&'p str],
    node: &'a RadixNode,
    bindings: &mut SmallVec<[(&'a str, BindingValue<'s, 'p>); 8]>,
) -> Option<&'a Arc<RouteNode>> {
    if segments.is_empty() {
        return node.route.as_ref();
    }
    let seg = segments[0];
    let rest = &segments[1..];

    // 1. Exact static segment: always preferred.
    if let Some(child) = node.static_child(seg)
        && let Some(r) = match_segments(rest, child, bindings)
    {
        return Some(r);
    }

    // 2. Dynamic parameters, in insertion order.
    for (name, child) in &node.param_children {
        bindings.push((name.as_ref(), BindingValue::Slice(seg)));
        if let Some(r) = match_segments(rest, child, bindings) {
            return Some(r);
        }
        bindings.pop();
    }

    // 3. Optional parameters: first try consuming the segment...
    for (name, child) in &node.optional_children {
        bindings.push((name.as_ref(), BindingValue::Slice(seg)));
        if let Some(r) = match_segments(rest, child, bindings) {
            return Some(r);
        }
        bindings.pop();
    }
    // ...then try skipping it.
    for (_name, child) in &node.optional_children {
        if let Some(r) = match_segments(segments, child, bindings) {
            return Some(r);
        }
    }

    // 4. Splat consumes everything that is left.
    if let Some(child) = &node.splat_child
        && let Some(route) = &child.route
    {
        bindings.push(("*splat", BindingValue::Splat(segments)));
        return Some(route);
    }

    None
}

/// Errors returned by [`RouteTree::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteTreeError {
    /// A route references a parent id that is not present in the tree.
    UnknownParent { route_id: String, parent: String },
    /// Following `parent` links never reaches the root (cycle or orphan).
    UnreachableFromRoot { route_id: String },
    /// Two routes with the same kind (layout/layout or page/page) share a pattern.
    DuplicatePattern {
        pattern: String,
        route_ids: Vec<String>,
    },
}

impl std::fmt::Display for RouteTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteTreeError::UnknownParent { route_id, parent } => {
                write!(f, "route `{route_id}` references unknown parent `{parent}`")
            }
            RouteTreeError::UnreachableFromRoot { route_id } => {
                write!(
                    f,
                    "route `{route_id}` is not reachable from the root (parent cycle?)"
                )
            }
            RouteTreeError::DuplicatePattern { pattern, route_ids } => {
                write!(
                    f,
                    "duplicate route pattern `{pattern}` used by {route_ids:?}"
                )
            }
        }
    }
}

impl std::error::Error for RouteTreeError {}

// ---- RouteTree ----
#[derive(Clone)]
pub struct RouteTree {
    nodes: FxHashMap<String, Arc<RouteNode>>,
    children: FxHashMap<String, Vec<String>>,
    root_id: String,
    trie: RouteTrie,
}

impl RouteTree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: FxHashMap::default(),
            children: FxHashMap::default(),
            root_id: "__root__".into(),
            trie: RouteTrie::default(),
        }
    }

    /// Convenience constructor that builds a tree from an iterator of nodes.
    #[must_use]
    pub fn from_nodes(nodes: impl IntoIterator<Item = RouteNode>) -> Self {
        let mut tree = Self::new();
        for node in nodes {
            tree.add_route(node);
        }
        tree
    }

    /// Ancestor chain from the tree root down to `route_id` (inclusive).
    ///
    /// Zero-allocation walk: parent ids are borrowed through the node map,
    /// so the only allocation is the returned `Vec` itself.
    #[must_use]
    pub fn ancestors(&self, route_id: &str) -> Vec<&RouteNode> {
        let mut chain = Vec::new();
        let mut current = route_id;
        while let Some(node) = self.nodes.get(current) {
            chain.push(Arc::as_ref(node));
            if chain.len() > 100 {
                log::error!("Cycle detected at {route_id}");
                break;
            }
            match node.parent.as_deref() {
                Some(parent) => current = parent,
                None => break,
            }
        }
        chain.reverse();
        chain
    }

    pub fn add_route(&mut self, node: RouteNode) {
        let id = node.id.clone();
        if let Some(p) = &node.parent {
            self.children.entry(p.clone()).or_default().push(id.clone());
        }
        let shared = Arc::new(node);
        self.trie.insert(Arc::clone(&shared));
        self.nodes.insert(id, shared);
    }

    #[must_use]
    pub fn match_path(&self, path: &str) -> MatchResult {
        self.trie.match_path(path)
    }

    #[must_use]
    pub fn get_node(&self, id: &str) -> Option<&RouteNode> {
        self.nodes.get(id).map(Arc::as_ref)
    }

    #[must_use]
    pub fn children_of(&self, pid: &str) -> Option<&Vec<String>> {
        self.children.get(pid)
    }

    #[must_use]
    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &RouteNode> {
        self.nodes.values().map(Arc::as_ref)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Structural sanity check: every parent must exist, every route must be
    /// reachable from the root via `parent` links, and patterns must be unique
    /// among routes of the same kind (layout/index overlaps are legitimate).
    ///
    /// # Errors
    /// Returns the first [`RouteTreeError`] found (unknown parent, parent
    /// cycle, or duplicate pattern).
    pub fn validate(&self) -> Result<(), RouteTreeError> {
        for node in self.nodes.values() {
            // A route without a parent is its own root; otherwise climb the
            // parent chain until a root is reached, the chain is broken
            // (unknown id), or it cycles. A broken chain is reported as an
            // unknown-parent error, attributed to the node whose parent link
            // dangles (direct parent or any deeper ancestor).
            let mut reachable = node.parent.is_none();
            let mut seen = std::collections::HashSet::new();
            let mut dangling_child = &node.id;
            let mut cur: Option<&String> = node.parent.as_ref();
            while let Some(id) = cur {
                if id == &self.root_id {
                    reachable = true;
                    break;
                }
                match self.nodes.get(id) {
                    Some(n) if n.parent.is_none() => {
                        reachable = true;
                        break;
                    }
                    Some(n) => {
                        if !seen.insert(id.clone()) {
                            break;
                        }
                        cur = n.parent.as_ref();
                        dangling_child = id;
                    }
                    None => {
                        return Err(RouteTreeError::UnknownParent {
                            route_id: dangling_child.clone(),
                            parent: id.clone(),
                        });
                    }
                }
            }
            if !reachable {
                return Err(RouteTreeError::UnreachableFromRoot {
                    route_id: node.id.clone(),
                });
            }
        }
        let mut by_pattern: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for node in self.nodes.values() {
            by_pattern
                .entry(node.pattern.raw.as_str())
                .or_default()
                .push(&node.id);
        }
        for (pattern, ids) in by_pattern {
            if ids.len() > 1 {
                let kinds: Vec<bool> = ids
                    .iter()
                    .filter_map(|id| self.nodes.get(*id).map(|n| n.is_layout))
                    .collect();
                // Layout + page sharing a pattern is the normal nesting model.
                if !(kinds.len() == 2 && kinds[0] != kinds[1]) {
                    return Err(RouteTreeError::DuplicatePattern {
                        pattern: pattern.to_string(),
                        route_ids: ids.into_iter().map(str::to_string).collect(),
                    });
                }
            }
        }
        Ok(())
    }
}

impl Default for RouteTree {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for RouteTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteTree")
            .field("routes", &self.nodes.len())
            .field("root_id", &self.root_id)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_node(id: &str, pat: &str) -> RouteNode {
        RouteNode {
            id: id.into(),
            pattern: RoutePattern::parse(pat),
            parent: None,
            is_layout: false,
            is_index: false,
            has_loader: false,
            loader_stale_time: None,
            loader_gc_time: None,
            preload_stale_time: None,
            before_load: None,
            on_enter: None,
            on_leave: None,
            loader_deps: None,
            context_fn: None,
            meta: HashMap::new(),
        }
    }

    fn route_id_of(tree: &RouteTree, path: &str) -> String {
        tree.match_path(path).expect("should match").1.id.clone()
    }

    #[test]
    fn test_static() {
        let mut t = RouteTree::new();
        t.add_route(make_node("u", "/users"));
        let (_p, r) = t.match_path("/users").expect("match");
        assert_eq!(r.id, "u");
    }

    #[test]
    fn test_dyn() {
        let mut t = RouteTree::new();
        t.add_route(make_node("d", "/users/$id"));
        let (p, n) = t.match_path("/users/42").expect("match");
        assert_eq!(n.id, "d");
        assert_eq!(p.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn test_splat() {
        let mut t = RouteTree::new();
        t.add_route(make_node("s", "/docs/$"));
        let (p, n) = t.match_path("/docs/a/b").expect("match");
        assert_eq!(n.id, "s");
        assert_eq!(p.get("*splat"), Some(&"a/b".to_string()));
    }

    #[test]
    fn test_opt_consume() {
        let mut t = RouteTree::new();
        t.add_route(make_node("o", "/{-$id}"));
        let (p, n) = t.match_path("/42").expect("match");
        assert_eq!(n.id, "o");
        assert_eq!(p.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn test_opt_skip() {
        let t = RouteTree::from_nodes([make_node("o", "/{-$id}")]);
        assert!(t.match_path("/").is_none());
    }

    #[test]
    fn test_static_beats_dynamic_regardless_of_order() {
        // Register the dynamic route first; the static one must still win.
        let mut t = RouteTree::new();
        t.add_route(make_node("dyn", "/users/$id"));
        t.add_route(make_node("new", "/users/new"));
        assert_eq!(route_id_of(&t, "/users/new"), "new");
        assert_eq!(route_id_of(&t, "/users/42"), "dyn");
    }

    #[test]
    fn test_dynamic_beats_optional() {
        let mut t = RouteTree::new();
        t.add_route(make_node("opt", "/users/{-$id}"));
        t.add_route(make_node("dyn", "/users/$name"));
        assert_eq!(route_id_of(&t, "/users/alice"), "dyn");
    }

    #[test]
    fn test_optional_beats_splat() {
        let mut t = RouteTree::new();
        t.add_route(make_node("splat", "/users/$"));
        t.add_route(make_node("opt", "/users/{-$id}"));
        assert_eq!(route_id_of(&t, "/users/alice"), "opt");
        // Two segments: optional skips one, splat cannot match under /users/x/y
        // because optional is tried first and consumes nothing...
        assert_eq!(route_id_of(&t, "/users/a/b"), "splat");
    }

    #[test]
    fn test_splat_captures_multi_segment() {
        let mut t = RouteTree::new();
        t.add_route(make_node("s", "/files/$"));
        let (p, _) = t.match_path("/files/a/b/c").expect("match");
        assert_eq!(p.get("*splat"), Some(&"a/b/c".to_string()));
    }

    #[test]
    fn test_splat_at_root() {
        let mut t = RouteTree::new();
        t.add_route(make_node("s", "/$"));
        let (p, n) = t.match_path("/anything/goes/here").expect("match");
        assert_eq!(n.id, "s");
        assert_eq!(p.get("*splat"), Some(&"anything/goes/here".to_string()));
    }

    #[test]
    fn test_bracket_static() {
        let mut t = RouteTree::new();
        t.add_route(make_node("b", "/[api]"));
        assert_eq!(route_id_of(&t, "/api"), "b");
    }

    #[test]
    fn test_curly_dynamic() {
        let mut t = RouteTree::new();
        t.add_route(make_node("c", "/{$id}"));
        assert_eq!(route_id_of(&t, "/42"), "c");
    }

    #[test]
    fn test_curly_optional() {
        let mut t = RouteTree::new();
        t.add_route(make_node("c", "/{-$lang}/docs"));
        assert_eq!(route_id_of(&t, "/en/docs"), "c");
    }

    #[test]
    fn test_deep_nesting() {
        let mut t = RouteTree::new();
        t.add_route(make_node("l1", "/a"));
        t.add_route(make_node("l2", "/a/b"));
        t.add_route(make_node("l3", "/a/b/$c"));
        t.add_route(make_node("l4", "/a/b/$c/d"));
        assert_eq!(route_id_of(&t, "/a"), "l1");
        assert_eq!(route_id_of(&t, "/a/b"), "l2");
        assert_eq!(route_id_of(&t, "/a/b/x"), "l3");
        assert_eq!(route_id_of(&t, "/a/b/x/d"), "l4");
    }

    #[test]
    fn test_no_match_returns_none() {
        let mut t = RouteTree::new();
        t.add_route(make_node("a", "/a"));
        assert!(t.match_path("/b").is_none());
        assert!(t.match_path("/a/b").is_none());
        assert!(t.match_path("/").is_none());
    }

    #[test]
    fn test_root_path_matches_root_route() {
        let mut t = RouteTree::new();
        t.add_route(make_node("root", "/"));
        assert_eq!(route_id_of(&t, "/"), "root");
    }

    #[test]
    fn test_trailing_slash_equivalent() {
        let mut t = RouteTree::new();
        t.add_route(make_node("u", "/users"));
        assert_eq!(route_id_of(&t, "/users/"), "u");
    }

    #[test]
    fn test_segment_helpers() {
        let s = Segment::Static("x".into());
        let d = Segment::Dynamic { name: "id".into() };
        let o = Segment::Optional { name: "id".into() };
        let sp = Segment::Splat;
        assert!(s.is_static() && !s.is_dynamic());
        assert!(d.is_dynamic() && d.param_name() == Some("id"));
        assert!(o.is_optional() && o.param_name() == Some("id"));
        assert!(sp.is_splat() && sp.param_name().is_none());
    }

    #[test]
    fn test_pattern_rank_prefers_static() {
        let static_pat = RoutePattern::parse("/users/new");
        let dyn_pat = RoutePattern::parse("/users/$id");
        let splat_pat = RoutePattern::parse("/users/$");
        assert!(static_pat.rank > dyn_pat.rank);
        assert!(dyn_pat.rank > splat_pat.rank);
        assert_eq!(static_pat.raw, "/users/new");
        assert_eq!(static_pat.segments.len(), 2);
    }

    #[test]
    fn test_pattern_debug() {
        let pat = RoutePattern::parse("/a/$b");
        assert!(format!("{pat:?}").contains("/a/$b"));
    }

    #[test]
    fn test_ancestors_order_root_first() {
        let mut t = RouteTree::new();
        let mut root = make_node("__root__", "/");
        root.is_layout = true;
        let mut layout = make_node("users", "/users");
        layout.is_layout = true;
        layout.parent = Some("__root__".into());
        let mut leaf = make_node("leaf", "/users/$id");
        leaf.parent = Some("users".into());
        t.add_route(root);
        t.add_route(layout);
        t.add_route(leaf);
        let chain: Vec<String> = t.ancestors("leaf").iter().map(|n| n.id.clone()).collect();
        assert_eq!(chain, vec!["__root__", "users", "leaf"]);
    }

    #[test]
    fn test_ancestors_unknown_id_empty() {
        let t = RouteTree::new();
        assert!(t.ancestors("nope").is_empty());
    }

    #[test]
    fn test_children_of() {
        let mut t = RouteTree::new();
        let mut parent = make_node("p", "/users");
        parent.is_layout = true;
        let mut child = make_node("c", "/users/$id");
        child.parent = Some("p".into());
        t.add_route(parent);
        t.add_route(child);
        assert_eq!(
            t.children_of("p").map(std::vec::Vec::as_slice),
            Some(["c".to_string()].as_slice())
        );
        assert!(t.children_of("c").is_none());
    }

    #[test]
    fn test_get_node_and_root_id() {
        let mut t = RouteTree::new();
        t.add_route(make_node("a", "/a"));
        assert!(t.get_node("a").is_some());
        assert!(t.get_node("b").is_none());
        assert_eq!(t.root_id(), "__root__");
        assert_eq!(t.len(), 1);
        assert!(!t.is_empty());
    }

    #[test]
    fn test_validate_ok_for_nested() {
        let mut t = RouteTree::new();
        let mut root = make_node("__root__", "/");
        root.is_layout = true;
        let mut child = make_node("c", "/a");
        child.parent = Some("__root__".into());
        t.add_route(root);
        t.add_route(child);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn test_validate_unknown_parent() {
        let mut t = RouteTree::new();
        let mut child = make_node("c", "/a");
        child.parent = Some("ghost".into());
        t.add_route(child);
        assert_eq!(
            t.validate(),
            Err(RouteTreeError::UnknownParent {
                route_id: "c".into(),
                parent: "ghost".into()
            })
        );
    }

    #[test]
    fn test_validate_cycle() {
        let mut t = RouteTree::new();
        let mut a = make_node("a", "/a");
        a.parent = Some("b".into());
        let mut b = make_node("b", "/b");
        b.parent = Some("a".into());
        t.add_route(a);
        t.add_route(b);
        assert!(matches!(
            t.validate(),
            Err(RouteTreeError::UnreachableFromRoot { .. })
        ));
    }

    #[test]
    fn test_validate_duplicate_patterns() {
        let mut t = RouteTree::new();
        t.add_route(make_node("a", "/dup"));
        t.add_route(make_node("b", "/dup"));
        assert!(matches!(
            t.validate(),
            Err(RouteTreeError::DuplicatePattern { .. })
        ));
    }

    #[test]
    fn test_validate_allows_layout_page_overlap() {
        let mut t = RouteTree::new();
        let mut layout = make_node("users_layout", "/users");
        layout.is_layout = true;
        let index = make_node("users_index", "/users");
        t.add_route(layout);
        t.add_route(index);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn test_tree_is_debug_and_clone() {
        let mut t = RouteTree::new();
        t.add_route(make_node("a", "/a"));
        let cloned = t.clone();
        assert_eq!(route_id_of(&cloned, "/a"), "a");
        assert!(format!("{t:?}").contains("RouteTree"));
    }

    #[test]
    fn test_route_node_debug_fields() {
        let node = make_node("a", "/a");
        let dbg = format!("{node:?}");
        assert!(dbg.contains("id: \"a\""));
        assert!(dbg.contains("has_loader: false"));
    }

    #[test]
    fn test_node_with_timings_and_meta() {
        let mut node = make_node("a", "/a");
        node.has_loader = true;
        node.loader_stale_time = Some(Duration::from_secs(10));
        node.loader_gc_time = Some(Duration::from_secs(60));
        node.preload_stale_time = Some(Duration::from_secs(5));
        node.meta.insert("title".into(), serde_json::json!("A"));
        let mut t = RouteTree::new();
        t.add_route(node.clone());
        let stored = t.get_node("a").expect("stored");
        assert_eq!(stored.loader_stale_time, Some(Duration::from_secs(10)));
        assert_eq!(stored.meta.get("title"), Some(&serde_json::json!("A")));
    }

    #[test]
    fn test_before_load_invocation() {
        use futures::executor::block_on;
        let mut node = make_node("g", "/guarded");
        node.before_load = Some(Arc::new(|_ctx| Box::pin(async { BeforeLoadResult::Ok })));
        let mut t = RouteTree::new();
        t.add_route(node);
        let (_, stored) = t.match_path("/guarded").expect("match");
        let f = stored.before_load.as_ref().expect("before_load").clone();
        let result = block_on(f(BeforeLoadContext {
            params: HashMap::new(),
            search: serde_json::Value::Null,
            location: Location::new("/guarded"),
        }));
        assert!(matches!(result, BeforeLoadResult::Ok));
    }

    #[test]
    fn test_route_context_args() {
        let args = RouteContextArgs {
            parent_context: Some(serde_json::json!({"a": 1})),
            params: HashMap::new(),
            loader_data: Some(AnyData::new(42_u32)),
        };
        assert_eq!(args.parent_context, Some(serde_json::json!({"a": 1})));
    }

    // ---- Matcher regression tests (sorted static fan-out + backtracking) ----

    #[test]
    fn test_static_fanout_beyond_inline_capacity() {
        // More than 8 static children forces the sorted small-vec into heap
        // territory; binary search must still find every member.
        let mut t = RouteTree::new();
        for i in 0..24 {
            t.add_route(make_node(&format!("s{i}"), &format!("/seg{i}")));
        }
        for i in 0..24 {
            assert_eq!(route_id_of(&t, &format!("/seg{i}")), format!("s{i}"));
        }
        assert!(t.match_path("/seg24").is_none());
    }

    #[test]
    fn test_backtracking_rolls_back_bindings() {
        // `/a/$x/b` fails for a one-segment path; the tentative `$x` binding
        // must be rolled back so `/a/$y` reports `y`, never a stale `x`.
        let mut t = RouteTree::new();
        t.add_route(make_node("xb", "/a/$x/b"));
        t.add_route(make_node("y", "/a/$y"));
        let (params, r) = t.match_path("/a/1").expect("match");
        assert_eq!(r.id, "y");
        assert_eq!(params.get("y"), Some(&"1".to_string()));
        assert!(!params.contains_key("x"));
        let (params, r) = t.match_path("/a/1/b").expect("match");
        assert_eq!(r.id, "xb");
        assert_eq!(params.get("x"), Some(&"1".to_string()));
    }

    #[test]
    fn test_failed_match_allocates_no_params() {
        // No match: the matcher must return `None` (no params materialized).
        let t = RouteTree::from_nodes([make_node("a", "/a/$x")]);
        assert!(t.match_path("/a").is_none());
    }

    #[test]
    fn test_tree_error_display_variants() {
        let e = RouteTreeError::UnknownParent {
            route_id: "c".into(),
            parent: "ghost".into(),
        };
        assert_eq!(e.to_string(), "route `c` references unknown parent `ghost`");
        let e = RouteTreeError::UnreachableFromRoot {
            route_id: "x".into(),
        };
        assert_eq!(
            e.to_string(),
            "route `x` is not reachable from the root (parent cycle?)"
        );
        let e = RouteTreeError::DuplicatePattern {
            pattern: "/dup".into(),
            route_ids: vec!["a".into(), "b".into()],
        };
        assert_eq!(
            e.to_string(),
            "duplicate route pattern `/dup` used by [\"a\", \"b\"]"
        );
    }

    #[test]
    fn test_tree_default_and_all_nodes() {
        let tree = RouteTree::default();
        assert!(tree.is_empty());
        let mut tree = RouteTree::new();
        tree.add_route(make_node("a", "/a"));
        tree.add_route(make_node("b", "/b"));
        let ids: Vec<&str> = tree.all_nodes().map(|n| n.id.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"a") && ids.contains(&"b"));
    }

    #[test]
    fn test_duplicate_optional_name_reuses_child() {
        // Two routes sharing an optional name at the same level must reuse
        // the same trie child (the `position`-hit branch).
        let mut t = RouteTree::new();
        t.add_route(make_node("a", "/{-$lang}/en"));
        t.add_route(make_node("b", "/{-$lang}/de"));
        assert_eq!(route_id_of(&t, "/fr/en"), "a");
        assert_eq!(route_id_of(&t, "/fr/de"), "b");
    }

    #[test]
    fn test_ancestors_breaks_on_extreme_depth() {
        // A parent chain longer than the cycle guard logs and stops at the
        // guard instead of recursing forever.
        let mut t = RouteTree::new();
        for i in 0..120 {
            let mut n = make_node(&format!("n{i}"), &format!("/s{i}"));
            n.parent = (i > 0).then(|| format!("n{}", i - 1));
            t.add_route(n);
        }
        let chain = t.ancestors("n119");
        assert_eq!(chain.len(), 101);
        assert_eq!(chain[0].id, "n19");
    }

    #[test]
    fn test_validate_unknown_grandparent_is_unknown_parent() {
        // The dangling link sits one level above the checked node; the error
        // still names the node whose parent link dangles.
        let mut t = RouteTree::new();
        let mut child = make_node("child", "/child");
        child.parent = Some("mid".into());
        let mut mid = make_node("mid", "/mid");
        mid.parent = Some("ghost".into());
        t.add_route(child);
        t.add_route(mid);
        assert_eq!(
            t.validate(),
            Err(RouteTreeError::UnknownParent {
                route_id: "mid".into(),
                parent: "ghost".into()
            })
        );
    }

    #[test]
    fn test_validate_reachable_via_parentless_ancestor() {
        // The parent chain ends at a node whose id is not the canonical root
        // but has no parent: still reachable.
        let mut t = RouteTree::new();
        let mut base = make_node("base", "/base");
        base.is_layout = true;
        let mut child = make_node("child", "/base/child");
        child.parent = Some("base".into());
        t.add_route(base);
        t.add_route(child);
        assert!(t.validate().is_ok());
    }

    #[test]
    fn test_optional_skip_keeps_earlier_bindings() {
        // `/p/$id/{-$lang}/end` — matching "/p/9/end" exercises the
        // optional-skip path: `$id` binding survives, `lang` is absent.
        let mut t = RouteTree::new();
        t.add_route(make_node("r", "/p/$id/{-$lang}/end"));
        let (params, r) = t.match_path("/p/9/end").expect("match");
        assert_eq!(r.id, "r");
        assert_eq!(params.get("id"), Some(&"9".to_string()));
        assert!(!params.contains_key("lang"));
        // Consuming the optional also works.
        let (params, r) = t.match_path("/p/9/en/end").expect("match");
        assert_eq!(r.id, "r");
        assert_eq!(params.get("id"), Some(&"9".to_string()));
        assert_eq!(params.get("lang"), Some(&"en".to_string()));
    }
}
