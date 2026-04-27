use std::collections::HashMap;
use crate::blocker::{Blocker, BlockerId};
use crate::history::History;
use crate::location::{Location, NavigateOptions};
use crate::route_tree::{RouteNode, RouteTree};

#[derive(Debug, Clone)]
pub enum NavigationEffect {
    SpawnLoader { route_id: String, params: HashMap<String, String> },
    Redirect { to: String, replace: bool },
    NotFound { data: Option<serde_json::Value> },
    NotifyListeners,
}

pub struct RouterCore {
    history: History,
    route_tree: RouteTree,
    pub current_match: Option<(HashMap<String, String>, RouteNode)>,
    blockers: HashMap<BlockerId, Blocker>,
    next_blocker_id: BlockerId,
    pending_navigation: Option<Location>,
    pub not_found_data: Option<serde_json::Value>,
}

impl RouterCore {
    pub fn new(initial: Location, route_tree: RouteTree) -> Self {
        let current_match = route_tree.match_path(&initial.pathname).map(|(p,n)| (p, n.clone()));
        Self {
            history: History::new(initial),
            route_tree,
            current_match,
            blockers: HashMap::new(),
            next_blocker_id: 0,
            pending_navigation: None,
            not_found_data: None,
        }
    }

    pub fn navigate(&mut self, loc: Location, options: NavigateOptions) -> Vec<NavigationEffect> {
        let mut effects = Vec::new();
        if !options.ignore_blocker {
            let all_allow = self.blockers.values().all(|_| true);
            if !all_allow { self.pending_navigation = Some(loc); return effects; }
        }

        let (params, matched_node) = match self.route_tree.match_path(&loc.pathname) {
            Some((p,n)) => (p, n.clone()),
            None => {
                effects.push(NavigationEffect::NotFound { data: self.not_found_data.clone() });
                return effects;
            }
        };

        self.current_match = Some((params.clone(), matched_node.clone()));
        if options.replace { self.history.replace(loc.clone()); } else { self.history.push(loc.clone()); }

        if matched_node.has_loader {
            effects.push(NavigationEffect::SpawnLoader { route_id: matched_node.id.clone(), params });
        }
        effects.push(NavigationEffect::NotifyListeners);
        effects
    }

    pub fn current_location(&self) -> Location { self.history.current() }
    pub fn add_blocker(&mut self, b: Blocker) -> BlockerId { let id=self.next_blocker_id; self.next_blocker_id+=1; self.blockers.insert(id,b); id }
    pub fn remove_blocker(&mut self, id: &BlockerId) { self.blockers.remove(id); }
    pub fn proceed(&mut self) -> Option<Location> { self.pending_navigation.take() }
    pub fn is_blocked(&self) -> bool { self.pending_navigation.is_some() }
    pub fn history(&self) -> &History { &self.history }
    pub fn route_tree(&self) -> &RouteTree { &self.route_tree }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::Location;
    use crate::route_tree::{RouteNode, RoutePattern};

    fn node(id: &str, pat: &str) -> RouteNode {
        RouteNode {
            id: id.into(), pattern: RoutePattern::parse(pat), parent: None, is_layout: false, is_index: false,
            has_loader: false, loader_stale_time: None, loader_gc_time: None, preload_stale_time: None,
            before_load: None, on_enter: None, on_leave: None, loader_deps: None, context_fn: None, meta: HashMap::new()
        }
    }

    #[test]
    fn test_navigate_static() {
        let mut tree = RouteTree::new();
        tree.add_route(node("home", "/"));
        tree.add_route(node("about", "/about"));
        let mut core = RouterCore::new(Location::new("/"), tree);
        let eff = core.navigate(Location::new("/about"), NavigateOptions::default());
        assert_eq!(eff.len(), 1);
        assert!(matches!(eff[0], NavigationEffect::NotifyListeners));
        assert_eq!(core.current_match.unwrap().1.id, "about");
    }

    #[test]
    fn test_not_found() {
        let mut core = RouterCore::new(Location::new("/"), RouteTree::new());
        let eff = core.navigate(Location::new("/x"), NavigateOptions::default());
        assert_eq!(eff.len(), 1);
        assert!(matches!(eff[0], NavigationEffect::NotFound{..}));
    }
}
