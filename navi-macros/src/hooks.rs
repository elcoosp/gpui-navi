use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Hook to access the current route's parameters.
pub fn use_params(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        navi_core::context::consume::<<#route_ty as navi_router::RouteDef>::Params>(
            navi_core::context::consume::<gpui::WindowId>(
                gpui::WindowId(0)
            ).unwrap_or(gpui::WindowId(0))
        )
        .expect("Params not found in context")
    };
    expanded.into()
}

/// Hook to access the current route's search parameters.
pub fn use_search(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        navi_core::context::consume::<<#route_ty as navi_router::RouteDef>::Search>(
            gpui::WindowId(0)
        )
        .expect("Search not found in context")
    };
    expanded.into()
}

/// Hook to access the current route's loader data.
pub fn use_loader_data(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        navi_core::context::consume::<<#route_ty as navi_router::RouteDef>::LoaderData>(
            gpui::WindowId(0)
        )
        .expect("LoaderData not found in context")
    };
    expanded.into()
}

/// Hook to get a navigator for programmatic navigation.
pub fn use_navigate(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        navi_router::Navigator::new(gpui::WindowId(0))
    };
    expanded.into()
}

/// Hook to create a navigation blocker.
pub fn use_blocker(input: TokenStream) -> TokenStream {
    let should_block = parse_macro_input!(input as syn::Expr);
    let expanded = quote! {{
        let blocker = navi_router::Blocker::new(#should_block);
        blocker
    }};
    expanded.into()
}

/// Hook to check if back navigation is possible.
pub fn use_can_go_back(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        false
    };
    expanded.into()
}
