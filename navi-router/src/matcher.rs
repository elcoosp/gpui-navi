use crate::route_tree::RoutePattern;
use std::collections::HashMap;

#[derive(Clone)]
pub struct RouteMatcher {
    patterns: Vec<(RoutePattern, String, bool)>, // (pattern, route_id, is_index)
}

impl RouteMatcher {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub fn insert(&mut self, pattern: RoutePattern, route_id: String, is_index: bool) {
        self.patterns.push((pattern, route_id, is_index));
        // Sort by rank descending, then index routes first
        self.patterns.sort_by(|a, b| {
            b.0.rank.cmp(&a.0.rank).then_with(|| b.2.cmp(&a.2)) // true (index) > false (layout)
        });
    }

    pub fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, String)> {
        for (pattern, route_id, _) in &self.patterns {
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
