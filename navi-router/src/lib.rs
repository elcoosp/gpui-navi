pub mod blocker;
pub mod components;
pub mod history;
pub mod loader;
pub mod location;
pub mod matcher;
pub mod navigator;
pub mod redirect;
pub mod route_tree;
pub mod state;
pub mod validation;

pub use blocker::{Blocker, BlockerId};
pub use history::History;
pub use loader::LoaderError;
pub use location::{Location, NavigateOptions, ScrollIntoViewOptions, ViewTransitionOptions};
pub use matcher::RouteMatcher;
pub use navigator::Navigator;
pub use redirect::{NotFound, Redirect, not_found, redirect};
pub use route_tree::{RouteNode, RoutePattern, RouteTree, Segment};
pub use state::{RouterEvent, RouterState};
pub use validation::{ValidateSearch, ValidationError, ValidationResult};
/// Trait for route definitions. Each route type must implement this.
pub trait RouteDef: 'static {
    type Params: Clone + std::fmt::Debug + 'static;
    type Search: Clone + std::fmt::Debug + 'static;
    type LoaderData: Clone + std::fmt::Debug + 'static;

    fn path() -> &'static str;
}
