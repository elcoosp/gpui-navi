// Re-export all core types for backward compatibility
pub use navi_router_core::{
    Blocker, BlockerId, History, Location, MatchResult, NavOp, NavigateOptions, NavigationEffect,
    NotFound, Redirect, RouteNode, RoutePattern, RouteTree, RouterCore, ScrollIntoViewOptions,
    Segment, ValidateSearch, ValidationError, ValidationResult, ViewTransitionOptions, not_found,
    redirect,
};

// Nested search-param helpers, gated on the `nested-search` feature.
#[cfg(feature = "nested-search")]
pub use navi_router_core::location::{parse_query_nested, query_to_string};

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

// quoin re-exports: the framework-agnostic runtime seam navi builds on.
pub use quoin_core::sleep;

// Adapter-specific modules and types
pub mod components;
pub mod event_bus;
pub mod navigator;
pub mod state;

pub use components::{
    Awaited, Link, Outlet, PreloadType, RouterProvider, ScrollRestoration, SuspenseBoundary,
    register_route_component,
};
pub use navigator::Navigator;
pub use state::{
    AnyData, LoaderOutcome, LoaderState, NotFoundMode, RouteDef, RouterEvent, RouterHandle,
    RouterHost, RouterOptions, RouterState,
};
