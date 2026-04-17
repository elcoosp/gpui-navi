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
    let name_str = name.to_string();
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

    let (has_loader, loader_factory_impl) = if let Some(loader_closure) = loader_closure {
        let stale_time_expr = stale_time.unwrap_or_else(|| syn::parse_quote! { std::time::Duration::ZERO });
        let gc_time_expr = gc_time.unwrap_or_else(|| syn::parse_quote! { std::time::Duration::from_secs(300) });
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
                        .with("route", #name_str)
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
            ::navi_router::RouterState::update(cx, |state, _cx| {
                state.register_loader_factory(#name_str, Self::loader_factory(executor));
            });
        }
    } else {
        quote! {}
    };

    let component_registration = if let Some(comp_ty) = component_ty {
        quote! {
            ::navi_router::components::register_route_component(#name_str, |_cx| {
                ::gpui::Component::new(#comp_ty).into_any_element()
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

        impl ::navi_router::RouteDef for #name {
            type Params = #params_ty;
            type Search = #search_ty;
            type LoaderData = #data_ty;

            fn path() -> &'static str {
                #path
            }

            fn name() -> &'static str {
                #name_str
            }
        }

        impl #name {
            pub fn build_node() -> ::navi_router::RouteNode {
                let pattern = ::navi_router::RoutePattern::parse(#path);
                ::navi_router::RouteNode {
                    id: #name_str.to_string(),
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

            pub fn register(cx: &mut ::gpui::App) {
                #component_registration
                #register_loader_call
            }
        }
    };

    TokenStream::from(expanded)
}
