pub mod location;
pub mod history;
pub mod route_tree;
pub mod matcher;
pub mod state;
pub mod navigator;
pub mod blocker;
pub mod loader;
pub mod validation;
pub mod redirect;
pub mod components;

pub use location::{Location, NavigateOptions, ScrollIntoViewOptions, ViewTransitionOptions};
pub use history::History;
pub use route_tree::{RouteNode, RoutePattern, RouteTree, Segment};
pub use matcher::RouteMatcher;
pub use state::{RouterState, RouterEvent};
pub use navigator::Navigator;
pub use blocker::{Blocker, BlockerId};
pub use redirect::{Redirect, NotFound, redirect, not_found};
pub use validation::{ValidateSearch, ValidationError, ValidationResult};

/// Trait for route definitions. Each route type must implement this.
pub trait RouteDef: 'static {
    type Params: Clone + std::fmt::Debug + 'static;
    type Search: Clone + std::fmt::Debug + 'static;
    type LoaderData: Clone + std::fmt::Debug + 'static;

    fn path() -> &'static str;
}
