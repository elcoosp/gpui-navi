use std::collections::HashMap;
use std::collections::BTreeMap;
use crate::radix_tree::RouteTrie;

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
    pub fn param_name(&self) -> Option<&str> {
        match self {
            Segment::Dynamic { name, .. } => Some(name),
            Segment::Optional { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn is_static(&self) -> bool {
        matches!(self, Segment::Static(_))
    }

    pub fn is_dynamic(&self) -> bool {
        matches!(self, Segment::Dynamic { .. })
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, Segment::Optional { .. })
    }

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
    pub fn parse(pattern: &str) -> Self {
        let raw = pattern.to_string();
        let segments = Self::parse_segments(pattern);
        let rank = Self::compute_rank(&segments);
        Self { raw, segments, rank }
    }

    pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut params = HashMap::new();
        let mut path_idx = 0;

        for seg in &self.segments {
            match seg {
                Segment::Static(s) => {
                    if path_idx >= path_segments.len() || path_segments[path_idx] != s {
                        return None;
                    }
                    path_idx += 1;
                }
                Segment::Dynamic { name, prefix, suffix } => {
                    if path_idx >= path_segments.len() {
                        return None;
                    }
                    let part = path_segments[path_idx];
                    if let Some(pre) = prefix {
                        if !part.starts_with(pre) { return None; }
                    }
                    if let Some(suf) = suffix {
                        if !part.ends_with(suf) { return None; }
                    }
                    let start = prefix.as_ref().map_or(0, |p| p.len());
                    let end = suffix.as_ref().map_or(part.len(), |s| part.len() - s.len());
                    if start > end { return None; }
                    params.insert(name.clone(), part[start..end].to_string());
                    path_idx += 1;
                }
                Segment::Optional { name, prefix, suffix } => {
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
                Segment::Splat { prefix: _, suffix: _ } => {
                    let remaining = path_segments[path_idx..].join("/");
                    params.insert("*splat".to_string(), remaining);
                    path_idx = path_segments.len();
                }
            }
        }

        if path_idx != path_segments.len() {
            return None;
        }

        Some(params)
    }

    fn parse_segments(pattern: &str) -> Vec<Segment> {
        let parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let mut segments = Vec::new();

        for part in parts {
            if part.starts_with('[') && part.ends_with(']') {
                segments.push(Segment::Static(part[1..part.len()-1].to_string()));
                continue;
            }
            if part.starts_with("{-$") && part.ends_with('}') {
                let name = part[3..part.len()-1].to_string();
                segments.push(Segment::Optional { name, prefix: None, suffix: None });
                continue;
            }
            if part.starts_with("{$") && part.contains('}') {
                let close_brace = part.find('}').unwrap();
                let name = part[2..close_brace].to_string();
                let suffix = if close_brace + 1 < part.len() {
                    Some(part[close_brace+1..].to_string())
                } else {
                    None
                };
                segments.push(Segment::Dynamic { name, prefix: None, suffix });
                continue;
            }
            if part == "$" {
                segments.push(Segment::Splat { prefix: None, suffix: None });
                continue;
            }
            if part.starts_with('$') {
                let name = part[1..].to_string();
                segments.push(Segment::Dynamic { name, prefix: None, suffix: None });
                continue;
            }
            segments.push(Segment::Static(part.to_string()));
        }

        segments
    }

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
        let mut current_id = Some(route_id.to_string());
        while let Some(id) = current_id {
            if let Some(node) = self.nodes.get(&id) {
                chain.push(node);
                current_id = node.parent.clone();
            } else {
                break;
            }
        }
        chain.reverse();
        chain
    }

    pub fn add_route(&mut self, node: RouteNode) {
        let id = node.id.clone();
        if let Some(parent) = &node.parent {
            self.children.entry(parent.clone()).or_default().push(id.clone());
        }
        self.nodes.insert(id.clone(), node.clone());
        self.trie.insert(node);
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
