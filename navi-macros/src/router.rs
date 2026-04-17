use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result as SynResult, Token};

/// Parsed router definition input.
struct RouterDefInput {
    routes: Vec<Ident>,
}

impl Parse for RouterDefInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut routes = Vec::new();
        while !input.is_empty() {
            routes.push(input.parse()?);
            let _ = input.parse::<Token![,]>();
        }
        Ok(RouterDefInput { routes })
    }
}

/// Generate a Route enum and route tree builder from a list of route types.
pub fn define_router(input: TokenStream) -> TokenStream {
    let input = match syn::parse::<RouterDefInput>(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    let route_variants: Vec<&Ident> = input.routes.iter().collect();

    let add_route_calls: Vec<_> = input
        .routes
        .iter()
        .map(|route| {
            quote! {
                tree.add_route(#route::build_node());
            }
        })
        .collect();

    let expanded = quote! {
        /// The Route enum representing all possible routes.
        #[derive(Clone, Debug)]
        pub enum Route {
            #(
                #route_variants,
            )*
        }

        /// Build the route tree with all registered routes.
        pub fn build_route_tree() -> navi_router::RouteTree {
            let mut tree = navi_router::RouteTree::new();
            #(
                #add_route_calls
            )*
            tree
        }

        /// Build the full router state with an initial location.
        pub fn build_router(
            initial: navi_router::Location,
            window_id: gpui::WindowId,
        ) -> navi_router::RouterState {
            let tree = build_route_tree();
            navi_router::RouterState::new(initial, window_id, tree)
        }
    };

    expanded.into()
}
