use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use futures::future::BoxFuture;
use crate::location::Location;
use crate::redirect::{NotFound, Redirect};
use crate::state_types::AnyData;

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

pub enum BeforeLoadResult { Ok, Redirect(Redirect), NotFound(NotFound) }
pub type BeforeLoadFn = Arc<dyn Fn(BeforeLoadContext) -> BoxFuture<'static, BeforeLoadResult> + Send + Sync>;

#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    Static(String),
    Dynamic { name: String, prefix: Option<String>, suffix: Option<String> },
    Optional { name: String, prefix: Option<String>, suffix: Option<String> },
    Splat { prefix: Option<String>, suffix: Option<String> },
}

impl Segment {
    pub fn param_name(&self) -> Option<&str> { match self { Segment::Dynamic { name, .. } | Segment::Optional { name, .. } => Some(name), _ => None } }
    pub fn is_static(&self) -> bool { matches!(self, Segment::Static(_)) }
    pub fn is_dynamic(&self) -> bool { matches!(self, Segment::Dynamic { .. }) }
    pub fn is_optional(&self) -> bool { matches!(self, Segment::Optional { .. }) }
    pub fn is_splat(&self) -> bool { matches!(self, Segment::Splat { .. }) }
}

#[derive(Clone, Debug)]
pub struct RoutePattern { pub raw: String, pub segments: Vec<Segment>, pub rank: usize }

impl RoutePattern {
    pub fn parse(pattern: &str) -> Self {
        let raw = pattern.to_string();
        let segments = Self::parse_segments(pattern);
        let rank = Self::compute_rank(&segments);
        Self { raw, segments, rank }
    }

    fn parse_segments(pattern: &str) -> Vec<Segment> {
        let parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let mut segments = Vec::new();
        for part in parts {
            if part.starts_with('[') && part.ends_with(']') {
                segments.push(Segment::Static(part[1..part.len()-1].to_string()));
            } else if part.starts_with("{-$") && part.ends_with('}') {
                let name = part[3..part.len()-1].to_string();
                segments.push(Segment::Optional { name, prefix: None, suffix: None });
            } else if part.starts_with("{$") && part.contains('}') {
                let close = part.find('}').unwrap();
                let name = part[2..close].to_string();
                let suffix = if close+1 < part.len() { Some(part[close+1..].to_string()) } else { None };
                segments.push(Segment::Dynamic { name, prefix: None, suffix });
            } else if part == "$" {
                segments.push(Segment::Splat { prefix: None, suffix: None });
            } else if part.starts_with('$') {
                let name = part[1..].to_string();
                segments.push(Segment::Dynamic { name, prefix: None, suffix: None });
            } else {
                segments.push(Segment::Static(part.to_string()));
            }
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
        if has_splat { rank = rank.saturating_sub(5_000); }
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
    pub on_enter: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
    pub on_leave: Option<Arc<dyn Fn(&Location) + Send + Sync>>,
    pub loader_deps: Option<Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>>,
    pub context_fn: Option<Arc<dyn Fn(RouteContextArgs) -> serde_json::Value + Send + Sync>>,
    pub meta: HashMap<String, serde_json::Value>,
}

impl std::fmt::Debug for RouteNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteNode").field("id", &self.id).field("pattern", &self.pattern.raw)
         .field("is_layout", &self.is_layout).field("is_index", &self.is_index)
         .field("has_before_load", &self.before_load.is_some()).finish()
    }
}

// ---- Radix Trie ----
#[derive(Clone, Debug, Default)]
struct RadixNode {
    children: HashMap<String, RadixNode>,
    route: Option<RouteNode>,
    param_name: Option<String>,
    optional_param: Option<String>,
    is_splat: bool,
}

impl RadixNode { fn new() -> Self { Self::default() } }

#[derive(Clone, Debug, Default)]
struct RouteTrie { root: RadixNode }

impl RouteTrie {
    fn new() -> Self { Self { root: RadixNode::new() } }
    fn insert(&mut self, node: RouteNode) {
        let mut current = &mut self.root;
        for seg in &node.pattern.segments {
            match seg {
                Segment::Static(s) => { current = current.children.entry(s.clone()).or_insert_with(RadixNode::new); }
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

    fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, &RouteNode)> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.match_segments(&segments, &self.root)
    }

    fn match_segments<'a>(&'a self, segments: &[&str], node: &'a RadixNode) -> Option<(HashMap<String, String>, &'a RouteNode)> {
        if segments.is_empty() { return node.route.as_ref().map(|r| (HashMap::new(), r)); }
        let seg = segments[0]; let rest = &segments[1..];
        if let Some(child) = node.children.get(seg) { if let Some(r) = self.match_segments(rest, child) { return Some(r); } }
        for (key, child) in &node.children {
            if let Some(name) = key.strip_prefix(':') {
                let mut params = HashMap::new(); params.insert(name.to_string(), seg.to_string());
                if let Some((sub, route)) = self.match_segments(rest, child) { params.extend(sub); return Some((params, route)); }
            }
        }
        for (key, child) in &node.children {
            if let Some(opt) = key.strip_prefix('?') {
                let mut p1 = HashMap::new(); p1.insert(opt.to_string(), seg.to_string());
                if let Some((sub, route)) = self.match_segments(rest, child) { p1.extend(sub); return Some((p1, route)); }
                if let Some((sub, route)) = self.match_segments(segments, child) { return Some((sub, route)); }
            }
        }
        if let Some(child) = node.children.get("*") {
            if child.is_splat {
                let mut params = HashMap::new();
                params.insert("*splat".to_string(), segments.join("/"));
                return child.route.as_ref().map(|r| (params, r));
            }
        }
        None
    }
}

// ---- RouteTree ----
#[derive(Clone)]
pub struct RouteTree {
    nodes: BTreeMap<String, RouteNode>,
    children: HashMap<String, Vec<String>>,
    root_id: String,
    trie: RouteTrie,
}

impl RouteTree {
    pub fn new() -> Self {
        Self { nodes: BTreeMap::new(), children: HashMap::new(), root_id: "__root__".into(), trie: RouteTrie::new() }
    }

    pub fn ancestors(&self, route_id: &str) -> Vec<&RouteNode> {
        let mut chain = Vec::new();
        let mut cur = Some(route_id.to_string());
        while let Some(id) = cur {
            if let Some(node) = self.nodes.get(&id) {
                chain.push(node);
                if chain.len() > 100 { log::error!("Cycle detected at {}", route_id); break; }
                cur = node.parent.clone();
            } else { break; }
        }
        chain.reverse(); chain
    }

    pub fn add_route(&mut self, node: RouteNode) {
        let id = node.id.clone();
        if let Some(p) = &node.parent { self.children.entry(p.clone()).or_default().push(id.clone()); }
        self.trie.insert(node.clone());
        self.nodes.insert(id, node);
    }

    pub fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, &RouteNode)> { self.trie.match_path(path) }
    pub fn get_node(&self, id: &str) -> Option<&RouteNode> { self.nodes.get(id) }
    pub fn children_of(&self, pid: &str) -> Option<&Vec<String>> { self.children.get(pid) }
    pub fn root_id(&self) -> &str { &self.root_id }
    pub fn all_nodes(&self) -> impl Iterator<Item = &RouteNode> { self.nodes.values() }
}

impl Default for RouteTree { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    fn make_node(id: &str, pat: &str) -> RouteNode {
        RouteNode { id: id.into(), pattern: RoutePattern::parse(pat), parent: None, is_layout: false, is_index: false,
            has_loader: false, loader_stale_time: None, loader_gc_time: None, preload_stale_time: None,
            before_load: None, on_enter: None, on_leave: None, loader_deps: None, context_fn: None,
            meta: HashMap::new() }
    }

    #[test] fn test_static() { let mut t=RouteTree::new(); t.add_route(make_node("u","/users")); let (_p,r)=t.match_path("/users").unwrap(); assert_eq!(r.id,"u"); }
    #[test] fn test_dyn() { let mut t=RouteTree::new(); t.add_route(make_node("d","/users/$id")); let (p,n)=t.match_path("/users/42").unwrap(); assert_eq!(n.id,"d"); assert_eq!(p.get("id").unwrap(),"42"); }
    #[test] fn test_splat() { let mut t=RouteTree::new(); t.add_route(make_node("s","/docs/$")); let (p,n)=t.match_path("/docs/a/b").unwrap(); assert_eq!(n.id,"s"); assert_eq!(p.get("*splat").unwrap(),"a/b"); }
    #[test] fn test_opt_consume() { let mut t=RouteTree::new(); t.add_route(make_node("o","/{-$id}")); let (p,n)=t.match_path("/42").unwrap(); assert_eq!(n.id,"o"); assert_eq!(p.get("id").unwrap(),"42"); }
    #[test] fn test_opt_skip() { let mut t=RouteTree::new(); t.add_route(make_node("o","/{-$id}")); assert!(t.match_path("/").is_none()); }
}
