//! `navi-router-core` is the framework-agnostic heart of the Navi router:
//! route patterns, the matching trie, history, navigation effects, blockers,
//! redirects and search-param validation. It deliberately has no GPUI
//! dependency so it can be tested and reused headlessly.

pub mod blocker;
pub mod core;
pub mod history;
pub mod location;
pub mod redirect;
pub mod route_tree;
pub mod state_types;
pub mod validation;

pub use blocker::{Blocker, BlockerId};
pub use core::{NavOp, NavigationEffect, RouterCore};
pub use history::History;
pub use location::{Location, NavigateOptions, ScrollIntoViewOptions, ViewTransitionOptions};
pub use redirect::{NotFound, Redirect, not_found, redirect};
pub use route_tree::{
    BeforeLoadContext, BeforeLoadResult, MatchResult, RouteContextArgs, RouteNode, RoutePattern,
    RouteTree, RouteTreeError, Segment,
};
pub use state_types::AnyData;
pub use validation::{ValidateSearch, ValidationError, ValidationResult};
