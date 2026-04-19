use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use futures::future::BoxFuture;
use crate::location::Location;
use crate::redirect::{NotFound, Redirect};

/// Arguments passed to route context function.
#[derive(Clone)]
pub struct RouteContextArgs {
    pub parent_context: Option<serde_json::Value>,
    pub params: HashMap<String, String>,
    pub loader_data: Option<crate::state::AnyData>,
}

/// Context passed to `before_load` hooks.
pub struct BeforeLoadContext {
    pub params: HashMap<String, String>,
    pub search: serde_json::Value,
    pub location: Location,
}

/// Result of a `before_load` hook.
pub enum BeforeLoadResult {
    /// Proceed with navigation.
    Ok,
    /// Redirect to a different location.
    Redirect(Redirect),
    /// Trigger a 404 not found.
    NotFound(NotFound),
}

/// Type alias for a before-load function.
pub type BeforeLoadFn = Arc<
    dyn Fn(BeforeLoadContext) -> BoxFuture<'static, BeforeLoadResult> + Send + Sync,
>;

/// A segment of a route pattern.
#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    /// A static path segment, e.g., "users" in "/users/$id"
    Static(String),
    /// A dynamic segment with optional prefix/suffix, e.g., "$id" or "{$param}.ext"
    Dynamic {
        name: String,
        prefix: Option<String>,
        suffix: Option<String>,
    },
    /// An optional dynamic segment, e.g., "{-$param}"
    Optional {
        name: String,
        prefix: Option<String>,
        suffix: Option<String>,
    },
    /// A splat segment that matches the rest of the path, e.g., "$"
    Splat {
        prefix: Option<String>,
        suffix: Option<String>,
    },
}

impl Segment {
    /// Returns the name of the parameter if this is a dynamic or optional segment.
    pub fn param_name(&self) -> Option<&str> {
        match self {
            Segment::Dynamic { name, .. } => Some(name),
            Segment::Optional { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Returns true if this is a static segment.
    pub fn is_static(&self) -> bool {
        matches!(self, Segment::Static(_))
    }

    /// Returns true if this is a dynamic segment.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Segment::Dynamic { .. })
    }

    /// Returns true if this is an optional segment.
    pub fn is_optional(&self) -> bool {
        matches!(self, Segment::Optional { .. })
    }

    /// Returns true if this is a splat segment.
    pub fn is_splat(&self) -> bool {
        matches!(self, Segment::Splat { .. })
    }
}

/// A parsed route pattern with segments and a computed specificity rank.
#[derive(Clone, Debug)]
pub struct RoutePattern {
    pub raw: String,
    pub segments: Vec<Segment>,
    pub rank: usize,
}

impl RoutePattern {
    /// Parse a pattern string like "/users/$id" into segments with ranking.
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
        let parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let mut segments = Vec::new();

        for part in parts {
            // Handle escaped segments [x]
            if part.starts_with('[') && part.ends_with(']') {
                segments.push(Segment::Static(part[1..part.len() - 1].to_string()));
                continue;
            }

            // Handle optional parameters {-$param}
            if part.starts_with("{-$") && part.ends_with('}') {
                let name = part[3..part.len() - 1].to_string();
                segments.push(Segment::Optional {
                    name,
                    prefix: None,
                    suffix: None,
                });
                continue;
            }

            // Handle prefix/suffix patterns {$param}.ext
            if part.starts_with("{$") && part.contains('}') {
                let close_brace = part.find('}').unwrap();
                let name = part[2..close_brace].to_string();
                let suffix = if close_brace + 1 < part.len() {
                    Some(part[close_brace + 1..].to_string())
                } else {
                    None
                };
                segments.push(Segment::Dynamic {
                    name,
                    prefix: None,
                    suffix,
                });
                continue;
            }

            // Handle splat $
            if part == "$" {
                segments.push(Segment::Splat {
                    prefix: None,
                    suffix: None,
                });
                continue;
            }

            // Handle dynamic segments $param
            if part.starts_with('$') {
                let name = part[1..].to_string();
                segments.push(Segment::Dynamic {
                    name,
                    prefix: None,
                    suffix: None,
                });
                continue;
            }

            // Static segment
            segments.push(Segment::Static(part.to_string()));
        }

        segments
    }

    /// Compute specificity rank: higher = more specific.
    /// Depth is the most important factor, then static count, etc.
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
}

/// Represents a node in the route tree.
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
    pub on_enter: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
    pub on_leave: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
    pub loader_deps: Option<Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>>,
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
            .field("has_before_load", &self.before_load.is_some())
            .finish()
    }
}

// ----------------------------------------------------------------------
// Radix Trie for fast route matching
// ----------------------------------------------------------------------

/// A node in the radix tree.
#[derive(Clone, Debug, Default)]
struct RadixNode {
    children: HashMap<String, RadixNode>,
    route: Option<RouteNode>,
    param_name: Option<String>,      // for dynamic segments like :id
    optional_param: Option<String>,  // for optional segments like ?id
    is_splat: bool,                  // for * segments
}

impl RadixNode {
    fn new() -> Self {
        Self::default()
    }
}

/// A custom radix tree for fast route matching.
#[derive(Clone, Debug, Default)]
struct RouteTrie {
    root: RadixNode,
}

impl RouteTrie {
    fn new() -> Self {
        Self { root: RadixNode::new() }
    }

    /// Insert a route into the trie.
    fn insert(&mut self, node: RouteNode) {
        let segments = &node.pattern.segments;
        let mut current = &mut self.root;

        for seg in segments {
            match seg {
                Segment::Static(s) => {
                    current = current.children.entry(s.clone()).or_insert_with(RadixNode::new);
                }
                Segment::Dynamic { name, .. } => {
                    let key = format!(":{}", name);
                    let child = current.children.entry(key).or_insert_with(RadixNode::new);
                    child.param_name = Some(name.clone());
                    current = child;
                }
                Segment::Optional { name, .. } => {
                    let key = format!("?{}", name);
                    let child = current.children.entry(key).or_insert_with(RadixNode::new);
                    child.optional_param = Some(name.clone());
                    current = child;
                }
                Segment::Splat { .. } => {
                    let key = "*".to_string();
                    let child = current.children.entry(key).or_insert_with(RadixNode::new);
                    child.is_splat = true;
                    current = child;
                }
            }
        }
        current.route = Some(node);
    }

    /// Match a path and extract parameters.
    fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, &RouteNode)> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.match_segments(&segments, &self.root)
    }

    fn match_segments<'a>(
        &'a self,
        segments: &[&str],
        node: &'a RadixNode,
    ) -> Option<(HashMap<String, String>, &'a RouteNode)> {
        if segments.is_empty() {
            return node.route.as_ref().map(|r| (HashMap::new(), r));
        }

        let segment = segments[0];
        let remaining = &segments[1..];

        // Try static match first
        if let Some(child) = node.children.get(segment) {
            if let Some((params, route)) = self.match_segments(remaining, child) {
                return Some((params, route));
            }
        }

        // Try dynamic segments (stored with prefix ':')
        for (key, child) in &node.children {
            if let Some(param_name) = key.strip_prefix(':') {
                let mut params = HashMap::new();
                params.insert(param_name.to_string(), segment.to_string());
                if let Some((sub_params, route)) = self.match_segments(remaining, child) {
                    params.extend(sub_params);
                    return Some((params, route));
                }
            }
        }

        // Try optional segments (stored with prefix '?')
        for (key, child) in &node.children {
            if let Some(opt_name) = key.strip_prefix('?') {
                // Option 1: consume
                let mut params1 = HashMap::new();
                params1.insert(opt_name.to_string(), segment.to_string());
                if let Some((sub_params, route)) = self.match_segments(remaining, child) {
                    params1.extend(sub_params);
                    return Some((params1, route));
                }
                // Option 2: skip
                if let Some((sub_params, route)) = self.match_segments(segments, child) {
                    return Some((sub_params, route));
                }
            }
        }

        // Try splat segment (key "*")
        if let Some(child) = node.children.get("*") {
            if child.is_splat {
                let mut params = HashMap::new();
                let remaining_path = segments.join("/");
                params.insert("*splat".to_string(), remaining_path);
                return child.route.as_ref().map(|r| (params, r));
            }
        }

        None
    }
}

// ----------------------------------------------------------------------
// RouteTree using RouteTrie + parent/children relationships
// ----------------------------------------------------------------------

/// The route tree holding all registered routes.
#[derive(Clone)]
pub struct RouteTree {
    nodes: BTreeMap<String, RouteNode>,
    children: HashMap<String, Vec<String>>,
    root_id: String,
    trie: RouteTrie,
}

impl RouteTree {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            children: HashMap::new(),
            root_id: "__root__".to_string(),
            trie: RouteTrie::new(),
        }
    }

    pub fn ancestors(&self, route_id: &str) -> Vec<&RouteNode> {
        let mut chain = Vec::new();
        let mut depth = 0;
        const MAX_DEPTH: usize = 100;
        let mut current_id = Some(route_id.to_string());
        while let Some(id) = current_id {
            if let Some(node) = self.nodes.get(&id) {
                chain.push(node);
                // Cycle detection
                if chain.len() > 100 {
                    log::error!("RouteTree::ancestors exceeded depth limit (100) - probable cycle involving {}", route_id);
                    break;
                }
                current_id = node.parent.clone();
            depth += 1;
            if depth > MAX_DEPTH {
                log::error!("RouteTree::ancestors exceeded depth limit ({}) - probable cycle involving {}", MAX_DEPTH, route_id);
                break;
            }
            } else {
                log::warn!("RouteTree::ancestors: node '{}' not found", id);
                break;
            }
        }
        chain.reverse();
        chain
    }

    pub fn add_route(&mut self, node: RouteNode) {
        let id = node.id.clone();
        if let Some(parent) = &node.parent {
            self.children
                .entry(parent.clone())
                .or_default()
                .push(id.clone());
        }
        self.trie.insert(node.clone());
        self.nodes.insert(id, node);
    }

    pub fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, &RouteNode)> {
        self.trie.match_path(path)
    }

    pub fn get_node(&self, id: &str) -> Option<&RouteNode> {
        self.nodes.get(id)
    }

    pub fn children_of(&self, parent_id: &str) -> Option<&Vec<String>> {
        self.children.get(parent_id)
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &RouteNode> {
        self.nodes.values()
    }
}

impl Default for RouteTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, pattern: &str) -> RouteNode {
        RouteNode {
            id: id.to_string(),
            pattern: RoutePattern::parse(pattern),
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
            meta: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_static_match() {
        let mut tree = RouteTree::new();
        tree.add_route(make_node("users", "/users"));
        let (params, matched) = tree.match_path("/users").unwrap();
        assert_eq!(matched.id, "users");
        assert!(params.is_empty());
    }

    #[test]
    fn test_dynamic_match() {
        let mut tree = RouteTree::new();
        tree.add_route(make_node("user_detail", "/users/$id"));
        let (params, matched) = tree.match_path("/users/42").unwrap();
        assert_eq!(matched.id, "user_detail");
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    fn test_splat_match() {
        let mut tree = RouteTree::new();
        tree.add_route(make_node("docs", "/docs/$"));
        let (params, matched) = tree.match_path("/docs/getting-started/intro").unwrap();
        assert_eq!(matched.id, "docs");
        assert_eq!(params.get("*splat").unwrap(), "getting-started/intro");
    }

    #[test]
    fn test_optional_match_consumed() {
        let mut tree = RouteTree::new();
        tree.add_route(make_node("optional", "/{-$id}"));
        let (params, matched) = tree.match_path("/42").unwrap();
        assert_eq!(matched.id, "optional");
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    #[test]
    fn test_optional_match_skipped() {
        let mut tree = RouteTree::new();
        tree.add_route(make_node("optional", "/{-$id}"));
        let result = tree.match_path("/");
        assert!(result.is_none());
    }
}
