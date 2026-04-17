// navi-macros/src/route.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ExprClosure, Ident, LitBool, LitStr, Result as SynResult, Token, Type};

struct RouteDefInput {
    name: Ident,
    path: LitStr,
    params_ty: Option<Type>,
    search_ty: Option<Type>,
    data_ty: Option<Type>,
    loader_closure: Option<ExprClosure>,
    component_ty: Option<Type>,
    stale_time: Option<syn::Expr>,
    gc_time: Option<syn::Expr>,
    #[allow(dead_code)]
    preload_stale_time: Option<syn::Expr>,
    is_layout: Option<LitBool>,
    is_index: Option<LitBool>,
    parent: Option<LitStr>,
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
        let mut is_layout = None;
        let mut is_index = None;
        let mut parent = None;

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
                "is_layout" => is_layout = Some(input.parse()?),
                "is_index" => is_index = Some(input.parse()?),
                "parent" => parent = Some(input.parse()?),
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
            is_layout,
            is_index,
            parent,
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
    let component_ty = input.component_ty;
    let is_layout = input.is_layout.map(|b| b.value).unwrap_or(false);
    let is_index = input.is_index.map(|b| b.value).unwrap_or(false);
    let parent = input.parent.map(|s| s.value());

    let (has_loader, loader_factory_impl) = if let Some(ref loader_closure) = input.loader_closure {
        let loader_closure = loader_closure.clone();
        let stale_time_expr = input
            .stale_time
            .clone()
            .unwrap_or_else(|| syn::parse_quote! { std::time::Duration::ZERO });
        let gc_time_expr = input
            .gc_time
            .clone()
            .unwrap_or_else(|| syn::parse_quote! { std::time::Duration::from_secs(300) });
        let factory = quote! {
            pub fn loader_factory(executor: ::gpui::BackgroundExecutor) -> std::sync::Arc<
                dyn Fn(&std::collections::HashMap<String, String>) -> ::rs_query::Query<::navi_router::AnyData>
                + Send + Sync
            > {
                std::sync::Arc::new(move |params_map: &std::collections::HashMap<String, String>| {
                    let params: #params_ty = serde_json::from_value(
                        serde_json::to_value(params_map).unwrap()
                    ).expect("Failed to deserialize route params");
                    let params_clone = params.clone();
                    let loader = #loader_closure;
                    let executor = executor.clone();
                    let key = ::rs_query::QueryKey::new("navi_loader")
                        .with("route", stringify!(#name))
                        .with("params", serde_json::to_string(&params).unwrap());
                    ::rs_query::Query::new(key, move || {
                        let params = params_clone.clone();
                        let loader = loader.clone();
                        let executor = executor.clone();
                        async move {
                            let data = loader(params, executor).await
                                .map_err(|e| ::rs_query::QueryError::custom(e.to_string()))?;
                            Ok(::navi_router::AnyData(std::sync::Arc::new(data) as std::sync::Arc<dyn std::any::Any + Send + Sync>))
                        }
                    })
                    .stale_time(#stale_time_expr)
                    .gc_time(#gc_time_expr)
                    .structural_sharing(true)
                })
            }
        };
        (true, factory)
    } else {
        (false, quote! {})
    };

    let register_loader_call = if has_loader {
        quote! {
            let executor = cx.background_executor().clone();
            navi_router::RouterState::update(cx, |state, _cx| {
                state.register_loader_factory(Self::name(), Self::loader_factory(executor));
            });
        }
    } else {
        quote! {}
    };

    let component_registration = if let Some(comp_ty) = component_ty {
        quote! {
            navi_router::components::register_route_component(Self::name(), |_cx| {
                gpui::Component::new(#comp_ty).into_any_element()
            });
        }
    } else {
        quote! {}
    };

    let parent_field = if let Some(p) = parent {
        quote! { Some(#p.to_string()) }
    } else {
        quote! { None }
    };

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
                    parent: #parent_field,
                    is_layout: #is_layout,
                    is_index: #is_index,
                    has_loader: #has_loader,
                    loader_stale_time: None,
                    loader_gc_time: None,
                    preload_stale_time: None,
                }
            }

            #loader_factory_impl

            pub fn register(cx: &mut gpui::App) {
                #component_registration
                #register_loader_call
            }
        }
    };

    expanded.into()
}
