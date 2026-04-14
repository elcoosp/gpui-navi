use std::collections::HashMap;

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

    /// Match a path against this pattern, extracting parameters.
    pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut params = HashMap::new();
        let mut path_idx = 0;

        for seg in &self.segments {
            match seg {
                Segment::Static(s) => {
                    if path_idx >= path_segments.len() {
                        return None;
                    }
                    if path_segments[path_idx] != s {
                        return None;
                    }
                    path_idx += 1;
                }
                Segment::Dynamic {
                    name,
                    prefix,
                    suffix,
                } => {
                    if path_idx >= path_segments.len() {
                        return None;
                    }
                    let part = path_segments[path_idx];
                    if let Some(pre) = prefix {
                        if !part.starts_with(pre) {
                            return None;
                        }
                    }
                    if let Some(suf) = suffix {
                        if !part.ends_with(suf) {
                            return None;
                        }
                    }
                    // Extract the value between prefix and suffix
                    let start = prefix.as_ref().map_or(0, |p| p.len());
                    let end = suffix.as_ref().map_or(part.len(), |s| part.len() - s.len());
                    if start > end {
                        return None;
                    }
                    params.insert(name.clone(), part[start..end].to_string());
                    path_idx += 1;
                }
                Segment::Optional {
                    name,
                    prefix,
                    suffix,
                } => {
                    if path_idx < path_segments.len() {
                        let part = path_segments[path_idx];
                        let start = prefix.as_ref().map_or(0, |p| p.len());
                        let end = suffix.as_ref().map_or(part.len(), |s| part.len() - s.len());
                        if start <= end {
                            params.insert(name.clone(), part[start..end].to_string());
                            path_idx += 1;
                        } else {
                            params.insert(name.clone(), String::new());
                        }
                    } else {
                        params.insert(name.clone(), String::new());
                    }
                }
                Segment::Splat {
                    prefix,
                    suffix: _suffix,
                } => {
                    // Splat matches all remaining path segments
                    let remaining = path_segments[path_idx..].join("/");
                    if let Some(_pre) = prefix {
                        // If there's a prefix, check it
                    }
                    params.insert("*splat".to_string(), remaining);
                    path_idx = path_segments.len();
                }
            }
        }

        // All path segments must be consumed (unless we have optional/splat at end)
        if path_idx != path_segments.len() {
            // Check if remaining segments are all optional
            return None;
        }

        Some(params)
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
    /// Index routes rank highest, splat lowest.
    fn compute_rank(segments: &[Segment]) -> usize {
        let static_count = segments.iter().filter(|s| s.is_static()).count();
        let dynamic_count = segments.iter().filter(|s| s.is_dynamic()).count();
        let optional_count = segments.iter().filter(|s| s.is_optional()).count();
        let has_splat = segments.iter().any(|s| s.is_splat());

        // Ranking: more static = higher, fewer dynamic = higher, fewer optional = higher, no splat = higher
        let mut rank = static_count * 100;
        rank += (10 - dynamic_count.min(10)) * 10;
        rank += (10 - optional_count.min(10)) * 5;
        if !has_splat {
            rank += 50;
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
}

impl std::fmt::Debug for RouteNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteNode")
            .field("id", &self.id)
            .field("pattern", &self.pattern.raw)
            .field("is_layout", &self.is_layout)
            .field("is_index", &self.is_index)
            .finish()
    }
}

use std::collections::BTreeMap;

/// The route tree holding all registered routes.
#[derive(Clone)]
pub struct RouteTree {
    nodes: BTreeMap<String, RouteNode>,
    children: HashMap<String, Vec<String>>,
    root_id: String,
    matcher: crate::matcher::RouteMatcher,
}

impl RouteTree {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            children: HashMap::new(),
            root_id: "__root__".to_string(),
            matcher: crate::matcher::RouteMatcher::new(),
        }
    }

    pub fn add_route(&mut self, node: RouteNode) {
        let id = node.id.clone();
        let pattern = node.pattern.clone();
        if let Some(parent) = &node.parent {
            self.children
                .entry(parent.clone())
                .or_default()
                .push(id.clone());
        }
        self.nodes.insert(id.clone(), node);
        self.matcher.insert(pattern, id);
    }

    pub fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, &RouteNode)> {
        self.matcher
            .match_path(path)
            .and_then(|(params, id)| self.nodes.get(&id).map(|node| (params, node)))
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
