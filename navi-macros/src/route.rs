use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ExprClosure, Ident, LitStr, Result as SynResult, Token, Type};

struct RouteDefInput {
    name: Ident,
    path: LitStr,
    params_ty: Option<Type>,
    search_ty: Option<Type>,
    data_ty: Option<Type>,
    loader_closure: Option<ExprClosure>,
    #[allow(dead_code)]
    component_ty: Option<Type>,
    #[allow(dead_code)]
    stale_time: Option<syn::Expr>,
    #[allow(dead_code)]
    gc_time: Option<syn::Expr>,
    #[allow(dead_code)]
    preload_stale_time: Option<syn::Expr>,
}

impl Parse for RouteDefInput {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let name: Ident = input.parse()?;
        if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
        }

        let mut path = None;
        let mut params_ty = None;
        let mut search_ty = None;
        let mut data_ty = None;
        let mut loader_closure = None;
        let mut component_ty = None;
        let mut stale_time = None;
        let mut gc_time = None;
        let mut preload_stale_time = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![:] = input.parse()?;
            match key.to_string().as_str() {
                "path" => path = Some(input.parse()?),
                "params" => params_ty = Some(input.parse()?),
                "search" => search_ty = Some(input.parse()?),
                "data" => data_ty = Some(input.parse()?),
                "loader" => loader_closure = Some(input.parse()?),
                "component" => component_ty = Some(input.parse()?),
                "stale_time" => stale_time = Some(input.parse()?),
                "gc_time" => gc_time = Some(input.parse()?),
                "preload_stale_time" => preload_stale_time = Some(input.parse()?),
                _ => return Err(syn::Error::new(key.span(), format!("Unknown key: {}", key))),
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let path = path
            .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "path is required"))?;

        Ok(RouteDefInput {
            name,
            path,
            params_ty,
            search_ty,
            data_ty,
            loader_closure,
            component_ty,
            stale_time,
            gc_time,
            preload_stale_time,
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
    let data_ty = input.data_ty.unwrap_or_else(|| syn::parse_quote!(()));

    let (has_loader, loader_registration) = if let Some(ref loader_closure) = input.loader_closure {
        let loader_closure = loader_closure.clone();
        let register = quote! {
            {
                use std::sync::Arc;
                use navi_router::LoaderError;

                log::debug!("Registering loader for route: {}", stringify!(#name));
                navi_router::RouterState::update(cx, |state, _cx| {
                    state.register_loader(
                        <#name as navi_router::RouteDef>::name(),
                        Box::new(|params_map: &std::collections::HashMap<String, String>, executor: gpui::BackgroundExecutor, _cx: &mut gpui::App| {
                            log::debug!("Loader function invoked for {}", stringify!(#name));
                            let params: #params_ty = serde_json::from_value(
                                serde_json::to_value(params_map).unwrap()
                            ).unwrap();
                            let loader = #loader_closure;
                            let fut = loader(params, executor);
                            _cx.spawn(async move |_cx| {
                                fut.await
                                    .map(|data| Arc::new(data) as Arc<dyn std::any::Any + Send + Sync>)
                                    .map_err(|e| e)
                            })
                        }),
                    );
                });
            }
        };
        (true, register)
    } else {
        (false, quote! {})
    };

    let is_layout = false;
    let is_index = false;

    let expanded = quote! {
        pub struct #name;

        impl navi_router::RouteDef for #name {
            type Params = #params_ty;
            type Search = #search_ty;
            type LoaderData = #data_ty;

            fn path() -> &'static str {
                #path
            }

            fn name() -> &'static str {
                stringify!(#name)
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
                    loader_stale_time: None,
                    loader_gc_time: None,
                    preload_stale_time: None,
                }
            }

            pub fn register_loader(cx: &mut gpui::App) {
                #loader_registration
            }
        }
    };

    expanded.into()
}
