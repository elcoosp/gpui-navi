use crate::route_tree::{RouteNode, Segment};
use std::collections::HashMap;

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
pub struct RouteTrie {
    root: RadixNode,
}

impl RouteTrie {
    pub fn new() -> Self {
        Self { root: RadixNode::new() }
    }

    /// Insert a route into the trie.
    pub fn insert(&mut self, node: RouteNode) {
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
    pub fn match_path<'a>(&'a self, path: &str) -> Option<(HashMap<String, String>, &'a RouteNode)> {
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
                // Two possibilities: consume the segment or skip it
                // Option 1: consume
                let mut params1 = HashMap::new();
                params1.insert(opt_name.to_string(), segment.to_string());
                if let Some((sub_params, route)) = self.match_segments(remaining, child) {
                    params1.extend(sub_params);
                    return Some((params1, route));
                }
                // Option 2: skip (treat as if segment wasn't there)
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
                // Splat consumes all remaining segments, so we don't recurse further
                return child.route.as_ref().map(|r| (params, r));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route_tree::{RouteNode, RoutePattern};

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
        }
    }

    #[test]
    fn test_static_match() {
        let mut trie = RouteTrie::new();
        trie.insert(make_node("users", "/users"));
        let (params, matched) = trie.match_path("/users").unwrap();
        assert_eq!(matched.id, "users");
        assert!(params.is_empty());
    }

    #[test]
    fn test_dynamic_match() {
        let mut trie = RouteTrie::new();
        trie.insert(make_node("user_detail", "/users/$id"));
        let (params, matched) = trie.match_path("/users/42").unwrap();
        assert_eq!(matched.id, "user_detail");
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    fn test_splat_match() {
        let mut trie = RouteTrie::new();
        trie.insert(make_node("docs", "/docs/$"));
        let (params, matched) = trie.match_path("/docs/getting-started/intro").unwrap();
        assert_eq!(matched.id, "docs");
        assert_eq!(params.get("*splat").unwrap(), "getting-started/intro");
    }

    #[test]
    fn test_optional_match_consumed() {
        let mut trie = RouteTrie::new();
        trie.insert(make_node("optional", "/{-$id}"));
        let (params, matched) = trie.match_path("/42").unwrap();
        assert_eq!(matched.id, "optional");
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    fn test_optional_match_skipped() {
        let mut trie = RouteTrie::new();
        trie.insert(make_node("optional", "/{-$id}"));
        let (params, matched) = trie.match_path("/").unwrap();
        assert_eq!(matched.id, "optional");
        assert!(params.get("id").is_none() || params.get("id").unwrap() == "");
    }
}
