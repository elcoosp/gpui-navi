mod hooks;
mod route;
mod router;

use proc_macro::TokenStream;

/// Define a route with type-safe parameters, search validation, and loader configuration.
#[proc_macro]
pub fn define_route(input: TokenStream) -> TokenStream {
    route::define_route(input)
}

/// Hook to access the current route's parameters.
#[proc_macro]
pub fn use_params(input: TokenStream) -> TokenStream {
    hooks::use_params(input)
}

/// Hook to access the current route's search parameters.
#[proc_macro]
pub fn use_search(input: TokenStream) -> TokenStream {
    hooks::use_search(input)
}

/// Hook to access the current route's loader data.
#[proc_macro]
pub fn use_loader_data(input: TokenStream) -> TokenStream {
    hooks::use_loader_data(input)
}

/// Hook to get a navigator for programmatic navigation.
#[proc_macro]
pub fn use_navigate(input: TokenStream) -> TokenStream {
    hooks::use_navigate(input)
}

/// Hook to create a navigation blocker.
#[proc_macro]
pub fn use_blocker(input: TokenStream) -> TokenStream {
    hooks::use_blocker(input)
}

/// Hook to check if back navigation is possible.
#[proc_macro]
pub fn use_can_go_back(input: TokenStream) -> TokenStream {
    hooks::use_can_go_back(input)
}

/// Hook to get the current match (route id and params).
#[proc_macro]
pub fn use_match(input: TokenStream) -> TokenStream {
    hooks::use_match(input)
}

/// Hook to get the loader state (Idle/Loading/Ready/Error) for a route.
#[proc_macro]
pub fn use_loader_state(input: TokenStream) -> TokenStream {
    hooks::use_loader_state(input)
}

/// Define a router from a list of route types, generating the Route enum and route tree.
#[proc_macro]
pub fn define_router(input: TokenStream) -> TokenStream {
    router::define_router(input)
}
