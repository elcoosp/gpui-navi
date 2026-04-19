use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, LitBool, LitStr, Token, Type,
};

struct RouteDefInput {
    name: Ident,
    _comma: Token![,],
    fields: Punctuated<Field, Token![,]>,
}

struct Field {
    key: Ident,
    _colon: Token![:],
    value: FieldValue,
}

enum FieldValue {
    LitStr(LitStr),
    LitBool(LitBool),
    Type(Type),
    Expr(Expr),
}

impl Parse for RouteDefInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let _comma = input.parse()?;
        let fields = Punctuated::parse_terminated(input)?;
        Ok(RouteDefInput { name, _comma, fields })
    }
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Field {
            key: input.parse()?,
            _colon: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl Parse for FieldValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            Ok(FieldValue::LitStr(input.parse()?))
        } else if input.peek(LitBool) {
            Ok(FieldValue::LitBool(input.parse()?))
        } else if input.peek(Ident) && (input.peek2(Token![<]) || input.peek2(Token![::]) || input.peek2(Token![,]) || input.is_empty()) {
            Ok(FieldValue::Type(input.parse()?))
        } else {
            Ok(FieldValue::Expr(input.parse()?))
        }
    }
}

pub fn define_route(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as RouteDefInput);
    let name = input.name;
    let mut path = None;
    let mut params_ty = None;
    let mut search_ty = None;
    let mut data_ty = None;
    let mut loader_closure = None;
    let mut component_ty = None;
    let mut stale_time = None;
    let mut gc_time = None;
    let mut is_layout = None;
    let mut is_index = None;
    let mut parent = None;
    let mut before_load_closure = None;
    let mut on_enter = None;
    let mut on_leave = None;
    let mut loader_deps = None;

    for field in input.fields {
        let key_str = field.key.to_string();
        match key_str.as_str() {
            "path" => {
                if let FieldValue::LitStr(lit) = field.value {
                    path = Some(lit);
                }
            }
            "params" => {
                if let FieldValue::Type(ty) = field.value {
                    params_ty = Some(ty);
                }
            }
            "search" => {
                if let FieldValue::Type(ty) = field.value {
                    search_ty = Some(ty);
                }
            }
            "data" => {
                if let FieldValue::Type(ty) = field.value {
                    data_ty = Some(ty);
                }
            }
            "loader" => {
                if let FieldValue::Expr(expr) = field.value {
                    loader_closure = Some(expr);
                }
            }
            "component" => {
                if let FieldValue::Type(ty) = field.value {
                    component_ty = Some(ty);
                }
            }
            "stale_time" => {
                if let FieldValue::Expr(expr) = field.value {
                    stale_time = Some(expr);
                }
            }
            "gc_time" => {
                if let FieldValue::Expr(expr) = field.value {
                    gc_time = Some(expr);
                }
            }
            "is_layout" => {
                if let FieldValue::LitBool(lit) = field.value {
                    is_layout = Some(lit);
                }
            }
            "is_index" => {
                if let FieldValue::LitBool(lit) = field.value {
                    is_index = Some(lit);
                }
            }
            "parent" => {
                if let FieldValue::LitStr(lit) = field.value {
                    parent = Some(lit);
                }
            }
            "before_load" => {
                if let FieldValue::Expr(expr) = field.value {
                    before_load_closure = Some(expr);
                }
            }
            "on_enter" => {
                if let FieldValue::Expr(expr) = field.value {
                    on_enter = Some(expr);
                }
            }
            "on_leave" => {
                if let FieldValue::Expr(expr) = field.value {
                    on_leave = Some(expr);
                }
            }
            "loader_deps" => {
                if let FieldValue::Expr(expr) = field.value {
                    loader_deps = Some(expr);
                }
            }
            _ => {}
        }
    }

    let path = path.expect("path is required");
    let params_ty = params_ty.unwrap_or_else(|| syn::parse_quote!(()));
    let search_ty = search_ty.unwrap_or_else(|| syn::parse_quote!(()));
    let data_ty = data_ty.unwrap_or_else(|| syn::parse_quote!(()));
    let component_ty = component_ty;
    let is_layout = is_layout.map(|b| b.value).unwrap_or(false);
    let is_index = is_index.map(|b| b.value).unwrap_or(false);
    let parent = parent.map(|s| s.value());

    let before_load_impl = if let Some(before_load) = before_load_closure {
        quote! {
            pub fn before_load_fn() -> Option<::navi_router::route_tree::BeforeLoadFn> {
                Some(::std::sync::Arc::new(|ctx| {
                    let closure = #before_load;
                    ::futures::future::FutureExt::boxed(closure(ctx))
                }))
            }
        }
    } else {
        quote! {
            pub fn before_load_fn() -> Option<::navi_router::route_tree::BeforeLoadFn> { None }
        }
    };

    let on_enter_impl = on_enter.map(|e| quote! { Some(::std::sync::Arc::new(#e)) }).unwrap_or(quote! { None });
    let on_leave_impl = on_leave.map(|e| quote! { Some(::std::sync::Arc::new(#e)) }).unwrap_or(quote! { None });
    let loader_deps_impl = loader_deps.map(|e| quote! { Some(::std::sync::Arc::new(#e)) }).unwrap_or(quote! { None });

    let (has_loader, loader_factory_impl) = if let Some(loader_closure) = loader_closure {
        let stale_time_expr = stale_time.clone().unwrap_or_else(|| syn::parse_quote! { std::time::Duration::ZERO });
        let gc_time_expr = gc_time.clone().unwrap_or_else(|| syn::parse_quote! { std::time::Duration::from_secs(300) });
        let factory = quote! {
            pub fn loader_factory(executor: ::gpui::BackgroundExecutor) -> std::sync::Arc<
                dyn Fn(&std::collections::HashMap<String, String>) -> ::rs_query::Query<::navi_router::LoaderOutcome<::navi_router::AnyData>>
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
                        .with("route", <#name as ::navi_router::RouteDef>::name())
                        .with("params", serde_json::to_string(&params).unwrap());
                    ::rs_query::Query::new(key, move || {
                        let params = params_clone.clone();
                        let loader = loader.clone();
                        let executor = executor.clone();
                        async move {
                            let data = loader(params, executor).await
                                .map_err(|e| ::rs_query::QueryError::custom(e.to_string()))?;
                            Ok(::navi_router::LoaderOutcome::Data(::navi_router::AnyData(std::sync::Arc::new(data) as std::sync::Arc<dyn std::any::Any + Send + Sync>)))
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
                state.register_loader_factory(<Self as ::navi_router::RouteDef>::name(), Self::loader_factory(executor));
            });
        }
    } else {
        quote! {}
    };

    let component_registration = if let Some(comp_ty) = component_ty {
        quote! {
            navi_router::components::register_route_component(<Self as ::navi_router::RouteDef>::name(), |_cx| {
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

    let stale_time_impl = stale_time.map(|e| quote! { Some(#e) }).unwrap_or(quote! { None });
    let gc_time_impl = gc_time.map(|e| quote! { Some(#e) }).unwrap_or(quote! { None });

    let expanded = quote! {
        pub struct #name;

        impl ::navi_router::RouteDef for #name {
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
                let mut node = navi_router::RouteNode {
                    id: <Self as ::navi_router::RouteDef>::name().to_string(),
                    pattern,
                    parent: #parent_field,
                    is_layout: #is_layout,
                    is_index: #is_index,
                    has_loader: #has_loader,
                    loader_stale_time: #stale_time_impl,
                    loader_gc_time: #gc_time_impl,
                    preload_stale_time: None,
                    before_load: Self::before_load_fn(),
                    on_enter: #on_enter_impl,
                    on_leave: #on_leave_impl,
                    loader_deps: #loader_deps_impl,
                };
                node
            }

            #before_load_impl

            #loader_factory_impl

            pub fn register(cx: &mut gpui::App) {
                #component_registration
                #register_loader_call
            }
        }
    };

    TokenStream::from(expanded)
}
