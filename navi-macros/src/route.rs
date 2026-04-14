use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Result as SynResult, Token, Type};

/// Parsed route definition input.
struct RouteDefInput {
    name: Ident,
    path: LitStr,
    params_ty: Option<Type>,
    search_ty: Option<Type>,
    #[allow(dead_code)]
    component_ty: Option<Type>,
    #[allow(dead_code)]
    error_component_ty: Option<Type>,
    #[allow(dead_code)]
    pending_component_ty: Option<Type>,
    #[allow(dead_code)]
    not_found_component_ty: Option<Type>,
    stale_time: Option<syn::Expr>,
    gc_time: Option<syn::Expr>,
    preload_stale_time: Option<syn::Expr>,
    #[allow(dead_code)]
    pending_ms: Option<syn::Expr>,
    #[allow(dead_code)]
    pending_min_ms: Option<syn::Expr>,
    #[allow(dead_code)]
    wrap_in_suspense: Option<syn::LitBool>,
}

impl Parse for RouteDefInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let name: Ident = input.parse()?;

        // Consume optional comma after the name
        if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
        }

        let mut path = None;
        let mut params_ty = None;
        let mut search_ty = None;
        let mut component_ty = None;
        let mut error_component_ty = None;
        let mut pending_component_ty = None;
        let mut not_found_component_ty = None;
        let mut stale_time = None;
        let mut gc_time = None;
        let mut preload_stale_time = None;
        let mut pending_ms = None;
        let mut pending_min_ms = None;
        let mut wrap_in_suspense = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![:] = input.parse()?;
            match key.to_string().as_str() {
                "path" => {
                    path = Some(input.parse()?);
                }
                "params" => {
                    params_ty = Some(input.parse()?);
                }
                "search" => {
                    search_ty = Some(input.parse()?);
                }
                "component" => {
                    component_ty = Some(input.parse()?);
                }
                "error_component" => {
                    error_component_ty = Some(input.parse()?);
                }
                "pending_component" => {
                    pending_component_ty = Some(input.parse()?);
                }
                "not_found_component" => {
                    not_found_component_ty = Some(input.parse()?);
                }
                "stale_time" => {
                    stale_time = Some(input.parse()?);
                }
                "gc_time" => {
                    gc_time = Some(input.parse()?);
                }
                "preload_stale_time" => {
                    preload_stale_time = Some(input.parse()?);
                }
                "pending_ms" => {
                    pending_ms = Some(input.parse()?);
                }
                "pending_min_ms" => {
                    pending_min_ms = Some(input.parse()?);
                }
                "wrap_in_suspense" => {
                    wrap_in_suspense = Some(input.parse()?);
                }
                _ => {
                    return Err(syn::Error::new(key.span(), format!("Unknown key: {}", key)));
                }
            }
            // Consume optional comma after value
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(RouteDefInput {
            name,
            path: path.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "path is required")
            })?,
            params_ty,
            search_ty,
            component_ty,
            error_component_ty,
            pending_component_ty,
            not_found_component_ty,
            stale_time,
            gc_time,
            preload_stale_time,
            pending_ms,
            pending_min_ms,
            wrap_in_suspense,
        })
    }
}

pub fn define_route(input: TokenStream) -> TokenStream {
    let input = match syn::parse::<RouteDefInput>(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error().into(),
    };

    let name = &input.name;
    let path = &input.path;

    let params_ty = input.params_ty.unwrap_or_else(|| syn::parse_quote!(()));
    let search_ty = input.search_ty.unwrap_or_else(|| syn::parse_quote!(()));

    let loader_data_ty: Type = syn::parse_quote!(());
    let has_loader = false;
    let is_layout = false;
    let is_index = false;

    let loader_stale_time = input
        .stale_time
        .map(|expr| {
            quote! { Some(#expr) }
        })
        .unwrap_or_else(|| quote! { None });

    let loader_gc_time = input
        .gc_time
        .map(|expr| {
            quote! { Some(#expr) }
        })
        .unwrap_or_else(|| quote! { None });

    let preload_stale_time = input
        .preload_stale_time
        .map(|expr| {
            quote! { Some(#expr) }
        })
        .unwrap_or_else(|| quote! { None });

    let expanded = quote! {
        pub struct #name;

        impl navi_router::RouteDef for #name {
            type Params = #params_ty;
            type Search = #search_ty;
            type LoaderData = #loader_data_ty;

            fn path() -> &'static str {
                #path
            }
        }

        impl #name {
            pub fn build_node() -> navi_router::RouteNode {
                let pattern = navi_router::RoutePattern::parse(#path);
                navi_router::RouteNode {
                    id: stringify!(#name).to_string(),
                    pattern,
                    parent: None,
                    is_layout: #is_layout,
                    is_index: #is_index,
                    has_loader: #has_loader,
                    loader_stale_time: #loader_stale_time,
                    loader_gc_time: #loader_gc_time,
                    preload_stale_time: #preload_stale_time,
                }
            }
        }
    };

    expanded.into()
}
