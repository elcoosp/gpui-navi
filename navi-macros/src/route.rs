// navi-macros/src/route.rs

use proc_macro::TokenStream;
use quote::quote;
use darling::{FromMeta, ast::NestedMeta};
use syn::{Ident, LitBool, LitStr};

#[derive(Debug, FromMeta)]
struct RouteDefArgs {
    path: LitStr,
    #[darling(default)]
    params: Option<syn::Path>,
    #[darling(default)]
    search: Option<syn::Path>,
    #[darling(default)]
    data: Option<syn::Path>,
    #[darling(default)]
    loader: Option<syn::Expr>,
    #[darling(default)]
    component: Option<syn::Path>,
    #[darling(default)]
    stale_time: Option<syn::Expr>,
    #[darling(default)]
    gc_time: Option<syn::Expr>,
    #[darling(default)]
    preload_stale_time: Option<syn::Expr>,
    #[darling(default)]
    is_layout: Option<LitBool>,
    #[darling(default)]
    is_index: Option<LitBool>,
    #[darling(default)]
    parent: Option<LitStr>,
}

pub fn define_route(input: TokenStream) -> TokenStream {
    let input2: proc_macro2::TokenStream = input.into();

    // Split off the route name (first identifier)
    let mut iter = input2.clone().into_iter();
    let name_tt = match iter.next() {
        Some(t) => t,
        None => return darling::Error::custom("define_route! requires a route name").write_errors().into(),
    };
    let name: Ident = match syn::parse2(proc_macro2::TokenStream::from(name_tt)) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error().into(),
    };

    let remaining: proc_macro2::TokenStream = iter.collect();

    let attr_args = match NestedMeta::parse_meta_list(remaining) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let args = match RouteDefArgs::from_list(&attr_args) {
        Ok(a) => a,
        Err(e) => return e.write_errors().into(),
    };

    // Validation
    if args.loader.is_some() && args.data.is_none() {
        return quote::quote_spanned! { name.span() =>
            compile_error!("define_route!: `loader` requires `data` to be specified.");
        }.into();
    }

    expand_define_route(name, args).into()
}

fn expand_define_route(name: Ident, args: RouteDefArgs) -> proc_macro2::TokenStream {
    let path = &args.path;
    let params_ty = args.params.unwrap_or_else(|| syn::parse_quote!(()));
    let search_ty = args.search.unwrap_or_else(|| syn::parse_quote!(()));
    let data_ty = args.data.unwrap_or_else(|| syn::parse_quote!(()));
    let component_ty = args.component;
    let is_layout = args.is_layout.map(|b| b.value).unwrap_or(false);
    let is_index = args.is_index.map(|b| b.value).unwrap_or(false);
    let parent = args.parent.map(|s| s.value());

    let (has_loader, loader_factory_impl) = if let Some(ref loader_closure) = args.loader {
        let loader_closure = loader_closure.clone();
        let stale_time_expr = args
            .stale_time
            .clone()
            .unwrap_or_else(|| syn::parse_quote! { std::time::Duration::ZERO });
        let gc_time_expr = args
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

    quote! {
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
    }
}
