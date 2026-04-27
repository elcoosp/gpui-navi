// Re-export all core types for backward compatibility
pub use navi_router_core::{
    Blocker, BlockerId, History, Location, NavigateOptions, NotFound, Redirect, RouteNode,
    RoutePattern, RouteTree, ScrollIntoViewOptions, Segment, ValidateSearch, ValidationError,
    ValidationResult, ViewTransitionOptions, not_found, redirect,
};

// Re-export route_tree items that are used via path in macros / route files
pub mod route_tree {
    pub use navi_router_core::route_tree::BeforeLoadFn;
    pub use navi_router_core::route_tree::{
        BeforeLoadContext, BeforeLoadResult, RouteContextArgs, RouteNode, RoutePattern, RouteTree,
        Segment,
    };
}

// Re-export commonly used items at crate root too
pub use navi_router_core::route_tree::{BeforeLoadContext, BeforeLoadResult, RouteContextArgs};

// Adapter-specific modules and types
pub mod components;
#[cfg(feature = "nexum")]
pub mod deep_link;
pub mod event_bus;
pub mod navigator;
pub mod state;

pub use components::{Link, Outlet, RouterProvider, register_route_component};
pub use navigator::Navigator;
pub use state::{
    AnyData, LoaderOutcome, NotFoundMode, RouteDef, RouterEvent, RouterOptions, RouterState,
};
