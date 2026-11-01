use crate::history::History;
use crate::location::{Location, NavigateOptions};
use crate::route_tree::{RouteNode, RouteTree};
use std::collections::HashMap;
use std::sync::Arc;

/// One step of a navigation transaction (see [`RouterCore::transact`]).
#[derive(Debug, Clone)]
pub enum NavOp {
    /// Push a new history entry and navigate to it.
    Push(Location),
    /// Overwrite the current history entry and navigate to it.
    Replace(Location),
    /// Step back one entry (no-op at the start of the history).
    Back,
    /// Step forward one entry (no-op at the end of the history).
    Forward,
    /// Relative traversal, clamped to the valid range.
    Go(isize),
}

#[derive(Debug, Clone)]
pub enum NavigationEffect {
    SpawnLoader {
        route_id: String,
        params: HashMap<String, String>,
    },
    Redirect {
        to: String,
        replace: bool,
    },
    NotFound {
        data: Option<serde_json::Value>,
    },
    NotifyListeners,
}

pub struct RouterCore {
    history: History,
    route_tree: RouteTree,
    pub current_match: Option<(HashMap<String, String>, Arc<RouteNode>)>,
    pending_navigation: Option<Location>,
    pub not_found_data: Option<serde_json::Value>,
}

impl RouterCore {
    #[must_use]
    pub fn new(initial: Location, route_tree: RouteTree) -> Self {
        let current_match = route_tree.match_path(&initial.pathname);
        Self {
            history: History::new(initial),
            route_tree,
            current_match,
            pending_navigation: None,
            not_found_data: None,
        }
    }

    /// Match a path without navigating. Returns the resolved params and the
    /// `Arc`-shared node (no deep clone).
    #[must_use]
    pub fn match_path(&self, path: &str) -> Option<(HashMap<String, String>, Arc<RouteNode>)> {
        self.route_tree.match_path(path)
    }

    pub fn navigate(&mut self, loc: Location, options: &NavigateOptions) -> Vec<NavigationEffect> {
        let matched = self.match_path(&loc.pathname);
        self.navigate_with(loc, options, matched)
    }

    /// Navigate using an already-computed match, avoiding a second
    /// route-tree lookup for callers that matched ahead of time.
    pub fn navigate_with(
        &mut self,
        loc: Location,
        options: &NavigateOptions,
        matched: Option<(HashMap<String, String>, Arc<RouteNode>)>,
    ) -> Vec<NavigationEffect> {
        let mut effects = Vec::new();
        let Some((params, matched_node)) = matched else {
            effects.push(NavigationEffect::NotFound {
                data: self.not_found_data.clone(),
            });
            self.pending_navigation = None;
            return effects;
        };
        self.current_match = Some((params.clone(), matched_node.clone()));
        if options.replace {
            self.history.replace(loc);
        } else {
            self.history.push(loc);
        }
        if matched_node.has_loader {
            effects.push(NavigationEffect::SpawnLoader {
                route_id: matched_node.id.clone(),
                params,
            });
        }
        effects.push(NavigationEffect::NotifyListeners);
        self.pending_navigation = None;
        effects
    }

    #[must_use]
    pub fn current_location(&self) -> Location {
        self.history.current()
    }

    #[must_use]
    pub fn history(&self) -> &History {
        &self.history
    }

    // Mutable history operations. Traversals re-match the route tree so
    // `current_match` always describes the location that is actually shown.
    #[must_use]
    pub fn back(&mut self) -> bool {
        if self.history.back() {
            self.rematch_current();
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn forward(&mut self) -> bool {
        if self.history.forward() {
            self.rematch_current();
            true
        } else {
            false
        }
    }

    pub fn go(&mut self, delta: isize) {
        self.history.go(delta);
        self.rematch_current();
    }

    /// Re-resolve `current_match` against the history's current location.
    fn rematch_current(&mut self) {
        let loc = self.history.current();
        self.current_match = self.route_tree.match_path(&loc.pathname);
    }

    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.history.can_go_back()
    }

    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.history.can_go_forward()
    }

    #[must_use]
    pub fn route_tree(&self) -> &RouteTree {
        &self.route_tree
    }

    #[must_use]
    pub fn is_navigation_pending(&self) -> bool {
        self.pending_navigation.is_some()
    }

    pub fn set_pending(&mut self, loc: Location) {
        self.pending_navigation = Some(loc);
    }

    pub fn clear_pending(&mut self) {
        self.pending_navigation = None;
    }

    /// Apply a batch of navigation operations in one synchronous step
    /// (a "navigation transaction").
    ///
    /// All ops run against the history and route tree without yielding, so
    /// no render, listener callback or loader spawn can interleave between
    /// them; the returned effects describe the whole batch with exactly one
    /// [`NavigationEffect::NotifyListeners`] (a single re-render per
    /// transaction). Each op is matched against the route tree exactly like
    /// a regular navigation, and unmatched pushes/replaces produce a
    /// [`NavigationEffect::NotFound`] instead of mutating history.
    pub fn transact(&mut self, ops: &[NavOp]) -> Vec<NavigationEffect> {
        let mut effects = Vec::new();
        let mut notified = false;
        for op in ops {
            match op {
                NavOp::Push(loc) => {
                    let matched = self.match_path(&loc.pathname);
                    let options = NavigateOptions {
                        replace: false,
                        ..NavigateOptions::default()
                    };
                    effects.extend(
                        self.navigate_with(loc.clone(), &options, matched)
                            .into_iter()
                            .filter(|e| !matches!(e, NavigationEffect::NotifyListeners)),
                    );
                    notified = true;
                }
                NavOp::Replace(loc) => {
                    let matched = self.match_path(&loc.pathname);
                    let options = NavigateOptions {
                        replace: true,
                        ..NavigateOptions::default()
                    };
                    effects.extend(
                        self.navigate_with(loc.clone(), &options, matched)
                            .into_iter()
                            .filter(|e| !matches!(e, NavigationEffect::NotifyListeners)),
                    );
                    notified = true;
                }
                NavOp::Back => {
                    // Route through the wrappers so `current_match` is
                    // re-matched after the traversal.
                    if self.back() {
                        notified = true;
                    }
                }
                NavOp::Forward => {
                    if self.forward() {
                        notified = true;
                    }
                }
                NavOp::Go(delta) => {
                    self.go(*delta);
                    notified = true;
                }
            }
        }
        // One re-render for the whole batch.
        if notified {
            effects.push(NavigationEffect::NotifyListeners);
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route_tree::RoutePattern;

    fn node(id: &str, pat: &str) -> RouteNode {
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

    fn tree_with(routes: &[(&str, &str)]) -> RouteTree {
        let mut tree = RouteTree::new();
        for (id, pat) in routes {
            tree.add_route(node(id, pat));
        }
        tree
    }

    #[test]
    fn test_initial_match() {
        let core = RouterCore::new(Location::new("/about"), tree_with(&[("about", "/about")]));
        assert_eq!(core.current_match.as_ref().expect("match").1.id, "about");
    }

    #[test]
    fn test_initial_no_match() {
        let core = RouterCore::new(Location::new("/missing"), RouteTree::new());
        assert!(core.current_match.is_none());
    }

    #[test]
    fn test_navigate_static() {
        let mut core = RouterCore::new(
            Location::new("/"),
            tree_with(&[("home", "/"), ("about", "/about")]),
        );
        let eff = core.navigate(Location::new("/about"), &NavigateOptions::default());
        assert_eq!(eff.len(), 1);
        assert!(matches!(eff[0], NavigationEffect::NotifyListeners));
        assert_eq!(core.current_match.as_ref().expect("match").1.id, "about");
        assert_eq!(core.current_location().pathname, "/about");
    }

    #[test]
    fn test_navigate_not_found() {
        let mut core = RouterCore::new(Location::new("/"), RouteTree::new());
        let eff = core.navigate(Location::new("/x"), &NavigateOptions::default());
        assert_eq!(eff.len(), 1);
        assert!(matches!(eff[0], NavigationEffect::NotFound { .. }));
        assert!(core.current_match.is_none());
    }

    #[test]
    fn test_navigate_with_loader_spawns_effect() {
        let mut tree = tree_with(&[("data", "/data")]);
        let mut n = node("data", "/data");
        n.has_loader = true;
        tree.add_route(n);
        let mut core = RouterCore::new(Location::new("/"), tree);
        let eff = core.navigate(Location::new("/data"), &NavigateOptions::default());
        assert_eq!(eff.len(), 2);
        assert!(matches!(
            eff[0],
            NavigationEffect::SpawnLoader { ref route_id, .. } if route_id == "data"
        ));
        assert!(matches!(eff[1], NavigationEffect::NotifyListeners));
    }

    #[test]
    fn test_navigate_with_params() {
        let mut core = RouterCore::new(Location::new("/"), tree_with(&[("u", "/users/$id")]));
        let eff = core.navigate(Location::new("/users/9"), &NavigateOptions::default());
        assert!(matches!(eff[0], NavigationEffect::NotifyListeners));
        let (params, _) = core.current_match.expect("match");
        assert_eq!(params.get("id"), Some(&"9".to_string()));
    }

    #[test]
    fn test_replace_vs_push() {
        let mut core = RouterCore::new(
            Location::new("/"),
            tree_with(&[("home", "/"), ("a", "/a"), ("b", "/b")]),
        );
        // Replacing the initial entry does not grow the history.
        core.navigate(
            Location::new("/b"),
            &NavigateOptions {
                replace: true,
                ..NavigateOptions::default()
            },
        );
        assert!(
            !core.can_go_back(),
            "replace of the initial entry must not grow history"
        );
        assert_eq!(core.current_location().pathname, "/b");
        // A subsequent push grows it again.
        core.navigate(Location::new("/a"), &NavigateOptions::default());
        assert!(core.can_go_back());
        assert_eq!(core.current_location().pathname, "/a");
    }

    #[test]
    fn test_history_back_forward() {
        let mut core =
            RouterCore::new(Location::new("/"), tree_with(&[("home", "/"), ("a", "/a")]));
        core.navigate(Location::new("/a"), &NavigateOptions::default());
        assert!(core.back());
        assert_eq!(core.current_location().pathname, "/");
        assert!(core.forward());
        assert_eq!(core.current_location().pathname, "/a");
        assert!(!core.can_go_forward());
    }

    #[test]
    fn test_traversal_rematches_current_match() {
        // Regression: back/forward/go used to leave `current_match` pointing
        // at the previous route, so the UI kept rendering the stale route.
        let mut core =
            RouterCore::new(Location::new("/"), tree_with(&[("home", "/"), ("a", "/a")]));
        core.navigate(Location::new("/a"), &NavigateOptions::default());
        assert_eq!(core.current_match.as_ref().expect("match").1.id, "a");
        assert!(core.back());
        assert_eq!(core.current_match.as_ref().expect("match").1.id, "home");
        assert!(core.forward());
        assert_eq!(core.current_match.as_ref().expect("match").1.id, "a");
        core.go(-1);
        assert_eq!(core.current_match.as_ref().expect("match").1.id, "home");
    }

    #[test]
    fn test_go_clamps() {
        let mut core =
            RouterCore::new(Location::new("/"), tree_with(&[("home", "/"), ("a", "/a")]));
        core.navigate(Location::new("/a"), &NavigateOptions::default());
        core.go(-10);
        assert_eq!(core.current_location().pathname, "/");
        core.go(10);
        assert_eq!(core.current_location().pathname, "/a");
    }

    #[test]
    fn test_navigate_with_precomputed_match() {
        let mut core = RouterCore::new(Location::new("/"), tree_with(&[("a", "/a")]));
        let matched = core.match_path("/a");
        let eff = core.navigate_with(Location::new("/a"), &NavigateOptions::default(), matched);
        assert!(matches!(eff[0], NavigationEffect::NotifyListeners));
        assert_eq!(core.current_match.expect("match").1.id, "a");
    }

    #[test]
    fn test_navigate_with_none_is_not_found() {
        let mut core = RouterCore::new(Location::new("/"), tree_with(&[("a", "/a")]));
        core.not_found_data = Some(serde_json::json!({"hint": "gone"}));
        let eff = core.navigate_with(Location::new("/zz"), &NavigateOptions::default(), None);
        match &eff[0] {
            NavigationEffect::NotFound { data } => {
                assert_eq!(data.as_ref(), Some(&serde_json::json!({"hint": "gone"})));
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_pending_navigation_state() {
        let mut core = RouterCore::new(Location::new("/"), tree_with(&[("a", "/a")]));
        assert!(!core.is_navigation_pending());
        core.set_pending(Location::new("/a"));
        assert!(core.is_navigation_pending());
        core.clear_pending();
        assert!(!core.is_navigation_pending());
    }

    #[test]
    fn test_route_tree_accessor_and_debug() {
        let mut core = RouterCore::new(Location::new("/"), tree_with(&[("a", "/a")]));
        assert_eq!(core.route_tree().len(), 1);
        core.navigate(Location::new("/a"), &NavigateOptions::default());
        assert!(format!("{:?}", core.current_match).contains("\"a\""));
    }

    #[test]
    fn test_not_found_data_default_none() {
        let core = RouterCore::new(Location::new("/"), RouteTree::new());
        assert!(core.not_found_data.is_none());
    }

    #[test]
    fn test_transact_pushes_and_single_notify() {
        let mut core = RouterCore::new(
            Location::new("/"),
            tree_with(&[("home", "/"), ("a", "/a"), ("b", "/b")]),
        );
        let ops = [
            NavOp::Push(Location::new("/a")),
            NavOp::Push(Location::new("/b")),
        ];
        let eff = core.transact(&ops);
        // Two loader-less pushes produce exactly one NotifyListeners.
        assert_eq!(eff.len(), 1);
        assert!(matches!(eff[0], NavigationEffect::NotifyListeners));
        assert_eq!(core.current_location().pathname, "/b");
        assert_eq!(core.history().len(), 3);
        assert!(core.can_go_back());
    }

    #[test]
    fn test_transact_replace_and_traversal() {
        let mut core = RouterCore::new(
            Location::new("/"),
            tree_with(&[("home", "/"), ("a", "/a"), ("b", "/b")]),
        );
        let ops = [
            NavOp::Push(Location::new("/a")),
            NavOp::Replace(Location::new("/b")),
            NavOp::Back,
        ];
        let eff = core.transact(&ops);
        assert!(matches!(
            eff.last(),
            Some(NavigationEffect::NotifyListeners)
        ));
        assert_eq!(core.current_location().pathname, "/");
        // Replace kept the history length at 2 ("/" + "/b").
        assert_eq!(core.history().len(), 2);
        assert!(core.forward());
        assert_eq!(core.current_location().pathname, "/b");
    }

    #[test]
    fn test_transact_unmatched_push_is_not_found_and_skips_history() {
        let mut core =
            RouterCore::new(Location::new("/"), tree_with(&[("home", "/"), ("a", "/a")]));
        core.not_found_data = Some(serde_json::json!({"x": 1}));
        let ops = [NavOp::Push(Location::new("/missing"))];
        let eff = core.transact(&ops);
        assert!(matches!(eff[0], NavigationEffect::NotFound { .. }));
        // The failed push must not grow the history.
        assert_eq!(core.history().len(), 1);
        assert_eq!(core.current_location().pathname, "/");
    }

    #[test]
    fn test_transact_go_clamps_and_empty_ops() {
        let mut core =
            RouterCore::new(Location::new("/"), tree_with(&[("home", "/"), ("a", "/a")]));
        assert!(core.transact(&[]).is_empty());
        core.transact(&[NavOp::Push(Location::new("/a"))]);
        let eff = core.transact(&[NavOp::Go(-10), NavOp::Go(10), NavOp::Go(-1)]);
        assert_eq!(eff.len(), 1);
        assert_eq!(core.current_location().pathname, "/");
    }

    #[test]
    fn test_transact_spawn_loader_effects_kept() {
        let mut tree = tree_with(&[("data", "/data")]);
        let mut n = node("data", "/data");
        n.has_loader = true;
        tree.add_route(n);
        let mut core = RouterCore::new(Location::new("/"), tree);
        let eff = core.transact(&[NavOp::Push(Location::new("/data"))]);
        assert_eq!(eff.len(), 2);
        assert!(matches!(
            eff[0],
            NavigationEffect::SpawnLoader { ref route_id, .. } if route_id == "data"
        ));
        assert!(matches!(eff[1], NavigationEffect::NotifyListeners));
    }

    /// Deterministic xorshift RNG so the fuzz runs are reproducible. All
    /// arithmetic stays in `usize` (no narrowing/sign casts).
    struct Rng(usize);

    impl Rng {
        fn next_usize(&mut self) -> usize {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn below(&mut self, n: usize) -> usize {
            self.next_usize() % n
        }
    }

    /// Model-based property test: a random sequence of navigation ops must
    /// keep `RouterCore` consistent with a plain `Vec` + cursor model of
    /// browser history. Replaces an external fuzzing dependency; the seed is
    /// fixed so failures are reproducible.
    #[test]
    fn fuzz_navigation_matches_history_model() {
        let paths = ["/", "/a", "/b", "/users/1", "/users/2", "/missing"];
        let deltas = [-2_isize, -1, 0, 1, 2];
        let tree = tree_with(&[("home", "/"), ("a", "/a"), ("b", "/b"), ("u", "/users/$id")]);
        // xorshift requires a nonzero seed.
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for _round in 0..200 {
            let mut core = RouterCore::new(Location::new("/"), tree.clone());
            // Model of `history_navigation`: stack of locations + cursor.
            let mut model: Vec<String> = vec!["/".to_string()];
            let mut cursor: usize = 0;
            for _step in 0..60 {
                match rng.below(6) {
                    0 => {
                        let path = paths[rng.below(paths.len())];
                        let matched = core.match_path(path).is_some();
                        core.transact(&[NavOp::Push(Location::new(path))]);
                        if matched {
                            model.truncate(cursor + 1);
                            model.push(path.to_string());
                            cursor += 1;
                        }
                    }
                    1 => {
                        let path = paths[rng.below(paths.len())];
                        let matched = core.match_path(path).is_some();
                        core.transact(&[NavOp::Replace(Location::new(path))]);
                        if matched {
                            model[cursor] = path.to_string();
                        }
                    }
                    2 => {
                        let moved = core.transact(&[NavOp::Back]).len() == 1;
                        if moved {
                            cursor -= 1;
                        }
                    }
                    3 => {
                        let moved = core.transact(&[NavOp::Forward]).len() == 1;
                        if moved {
                            cursor += 1;
                        }
                    }
                    4 => {
                        let delta = deltas[rng.below(deltas.len())];
                        let new_cursor = cursor.saturating_add_signed(delta).min(model.len() - 1);
                        core.transact(&[NavOp::Go(delta)]);
                        cursor = new_cursor;
                    }
                    _ => {
                        // Invariant probes between ops.
                        assert_eq!(core.current_location().pathname, model[cursor]);
                        assert_eq!(core.history().len(), model.len());
                        assert_eq!(core.can_go_back(), cursor > 0);
                        assert_eq!(core.can_go_forward(), cursor + 1 < model.len());
                        // `current_match` must always describe the shown
                        // location, including after history traversal.
                        let loc = core.current_location();
                        assert_eq!(
                            core.current_match.as_ref().map(|(_, n)| n.id.clone()),
                            core.match_path(&loc.pathname).map(|(_, n)| n.id.clone()),
                        );
                    }
                }
            }
            assert_eq!(core.current_location().pathname, model[cursor]);
            assert_eq!(core.history().len(), model.len());
        }
    }
}
