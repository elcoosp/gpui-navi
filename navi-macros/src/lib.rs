mod hooks;
mod route;
mod router;

use proc_macro::TokenStream;

#[proc_macro]
pub fn define_route(input: TokenStream) -> TokenStream {
    route::define_route(input)
}

#[proc_macro]
pub fn use_params(input: TokenStream) -> TokenStream {
    hooks::use_params(input)
}

#[proc_macro]
pub fn use_search(input: TokenStream) -> TokenStream {
    hooks::use_search(input)
}

#[proc_macro]
pub fn use_loader_data(input: TokenStream) -> TokenStream {
    hooks::use_loader_data(input)
}

#[proc_macro]
pub fn use_navigate(input: TokenStream) -> TokenStream {
    hooks::use_navigate(input)
}

#[proc_macro]
pub fn use_blocker(input: TokenStream) -> TokenStream {
    hooks::use_blocker(input)
}

#[proc_macro]
pub fn use_can_go_back(input: TokenStream) -> TokenStream {
    hooks::use_can_go_back(input)
}

#[proc_macro]
pub fn use_match(input: TokenStream) -> TokenStream {
    hooks::use_match(input)
}

#[proc_macro]
pub fn use_loader_state(input: TokenStream) -> TokenStream {
    hooks::use_loader_state(input)
}

#[proc_macro]
pub fn use_route_context(input: TokenStream) -> TokenStream {
    hooks::use_route_context(input)
}

#[proc_macro]
pub fn use_matched_route(input: TokenStream) -> TokenStream {
    hooks::use_matched_route(input)
}

#[proc_macro]
pub fn define_router(input: TokenStream) -> TokenStream {
    router::define_router(input)
}
