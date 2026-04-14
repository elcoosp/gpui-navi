use crate::route_tree::RoutePattern;
use std::collections::HashMap;

/// Route matcher that stores patterns sorted by specificity rank.
pub struct RouteMatcher {
    patterns: Vec<(RoutePattern, String)>,
}

impl RouteMatcher {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub fn insert(&mut self, pattern: RoutePattern, route_id: String) {
        self.patterns.push((pattern, route_id));
        // Sort by rank (specificity) descending - most specific first
        self.patterns.sort_by(|a, b| b.0.rank.cmp(&a.0.rank));
    }

    pub fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, String)> {
        for (pattern, route_id) in &self.patterns {
            if let Some(params) = pattern.matches(path) {
                return Some((params, route_id.clone()));
            }
        }
        None
    }
}

impl Default for RouteMatcher {
    fn default() -> Self {
        Self::new()
    }
}
