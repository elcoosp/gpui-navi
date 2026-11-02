use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, Ident, LitBool, LitStr, Token, Type,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

struct RouteDefInput {
    name: Ident,
    fields: Punctuated<Field, Token![,]>,
}

struct Field {
    key: Ident,
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
        input.parse::<Token![,]>()?;
        let fields = Punctuated::parse_terminated(input)?;
        Ok(RouteDefInput { name, fields })
    }
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        // Parse the value according to the key so that paths like
        // `std::time::Duration::from_secs(30)` parse as expressions while
        // `params: MyParams` still parses as a type.
        let value = match key.to_string().as_str() {
            "path" | "parent" => FieldValue::LitStr(input.parse()?),
            "is_layout" | "is_index" => FieldValue::LitBool(input.parse()?),
            "params" | "search" | "data" | "component" => FieldValue::Type(input.parse()?),
            _ => FieldValue::Expr(input.parse()?),
        };
        Ok(Field { key, value })
    }
}

/// All accepted `define_route!` arguments with defaults applied.
struct RouteArgs {
    path: LitStr,
    params_ty: Type,
    search_ty: Type,
    data_ty: Type,
    loader: Option<Expr>,
    component: Option<Type>,
    stale_time: Option<Expr>,
    gc_time: Option<Expr>,
    is_layout: bool,
    is_index: bool,
    parent: Option<LitStr>,
    before_load: Option<Expr>,
    on_enter: Option<Expr>,
    on_leave: Option<Expr>,
    loader_deps: Option<Expr>,
    context_fn: Option<Expr>,
    meta: Option<Expr>,
}

fn parse_route_args(name: &Ident, fields: Punctuated<Field, Token![,]>) -> syn::Result<RouteArgs> {
    let mut path: Option<LitStr> = None;
    let mut params_ty: Option<Type> = None;
    let mut search_ty: Option<Type> = None;
    let mut data_ty: Option<Type> = None;
    let mut loader: Option<Expr> = None;
    let mut component: Option<Type> = None;
    let mut stale_time: Option<Expr> = None;
    let mut gc_time: Option<Expr> = None;
    let mut is_layout: Option<LitBool> = None;
    let mut is_index: Option<LitBool> = None;
    let mut parent: Option<LitStr> = None;
    let mut before_load: Option<Expr> = None;
    let mut on_enter: Option<Expr> = None;
    let mut on_leave: Option<Expr> = None;
    let mut loader_deps: Option<Expr> = None;
    let mut context_fn: Option<Expr> = None;
    let mut meta: Option<Expr> = None;

    macro_rules! assign {
        ($field:ident, $key:literal, $slot:ident, $variant:ident) => {
            if $field.key == $key {
                if let FieldValue::$variant(value) = $field.value {
                    $slot = Some(value);
                    continue;
                }
            }
        };
    }
    for field in fields {
        assign!(field, "path", path, LitStr);
        assign!(field, "parent", parent, LitStr);
        assign!(field, "params", params_ty, Type);
        assign!(field, "search", search_ty, Type);
        assign!(field, "data", data_ty, Type);
        assign!(field, "component", component, Type);
        assign!(field, "stale_time", stale_time, Expr);
        assign!(field, "gc_time", gc_time, Expr);
        assign!(field, "is_layout", is_layout, LitBool);
        assign!(field, "is_index", is_index, LitBool);
        assign!(field, "before_load", before_load, Expr);
        assign!(field, "on_enter", on_enter, Expr);
        assign!(field, "on_leave", on_leave, Expr);
        assign!(field, "loader_deps", loader_deps, Expr);
        assign!(field, "context", context_fn, Expr);
        assign!(field, "meta", meta, Expr);
        if field.key == "loader"
            && let FieldValue::Expr(expr) = field.value
        {
            loader = Some(expr);
            continue;
        }
        return Err(syn::Error::new(
            field.key.span(),
            format!("unknown `define_route!` field `{}`", field.key),
        ));
    }

    let Some(path) = path else {
        return Err(syn::Error::new(name.span(), "`path` is required"));
    };
    Ok(RouteArgs {
        path,
        params_ty: params_ty.unwrap_or_else(|| syn::parse_quote!(())),
        search_ty: search_ty.unwrap_or_else(|| syn::parse_quote!(())),
        data_ty: data_ty.unwrap_or_else(|| syn::parse_quote!(())),
        loader,
        component,
        stale_time,
        gc_time,
        is_layout: is_layout.is_some_and(|b| b.value),
        is_index: is_index.is_some_and(|b| b.value),
        parent,
        before_load,
        on_enter,
        on_leave,
        loader_deps,
        context_fn,
        meta,
    })
}

fn before_load_impl(args: &RouteArgs) -> TokenStream2 {
    let Some(before_load) = &args.before_load else {
        return quote! {
            pub fn before_load_fn() -> Option<::navi_router::route_tree::BeforeLoadFn> { None }
        };
    };
    quote! {
        pub fn before_load_fn() -> Option<::navi_router::route_tree::BeforeLoadFn> {
            Some(::std::sync::Arc::new(|ctx| {
                let closure = #before_load;
                // `Box::pin` instead of `futures::FutureExt::boxed` keeps
                // downstream crates free of a hard `futures` dependency.
                ::std::boxed::Box::pin(closure(ctx))
            }))
        }
    }
}

fn loader_factory_impl(name: &Ident, args: &RouteArgs) -> (bool, TokenStream2) {
    let Some(loader_closure) = &args.loader else {
        return (false, quote! {});
    };
    let stale_time_expr = args
        .stale_time
        .clone()
        .unwrap_or_else(|| syn::parse_quote! { std::time::Duration::ZERO });
    let gc_time_expr = args
        .gc_time
        .clone()
        .unwrap_or_else(|| syn::parse_quote! { std::time::Duration::from_secs(300) });
    let params_ty = &args.params_ty;
    let factory = quote! {
        pub fn loader_factory(executor: ::gpui::BackgroundExecutor) -> std::sync::Arc<
            dyn Fn(&std::collections::HashMap<String, String>) -> ::rs_query::Query<::navi_router::LoaderOutcome<::navi_router::AnyData>>
            + Send + Sync
        > {
            std::sync::Arc::new(move |params_map: &std::collections::HashMap<String, String>| {
                let params: #params_ty = if std::any::TypeId::of::<#params_ty>() == std::any::TypeId::of::<()>() {
                    serde_json::from_value(serde_json::Value::Null).unwrap()
                } else {
                    serde_json::from_value(
                        serde_json::to_value(params_map).unwrap()
                    ).expect("Failed to deserialize route params")
                };
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
}

fn optional_arc_impl(expr: Option<&Expr>) -> TokenStream2 {
    expr.map_or(
        quote! { None },
        |e| quote! { Some(::std::sync::Arc::new(#e)) },
    )
}

fn event_fns_impl(args: &RouteArgs) -> (TokenStream2, TokenStream2) {
    let enter = optional_arc_impl(args.on_enter.as_ref());
    let leave = optional_arc_impl(args.on_leave.as_ref());
    (enter, leave)
}

fn expand(name: &Ident, args: &RouteArgs) -> TokenStream2 {
    let path = &args.path;
    let params_ty = &args.params_ty;
    let search_ty = &args.search_ty;
    let data_ty = &args.data_ty;
    let parent_field = if let Some(p) = &args.parent {
        quote! { Some(#p.to_string()) }
    } else {
        quote! { None }
    };
    let is_layout = args.is_layout;
    let is_index = args.is_index;

    let (on_enter_impl, on_leave_impl) = event_fns_impl(args);
    let loader_deps_impl = optional_arc_impl(args.loader_deps.as_ref());
    let context_fn_impl = optional_arc_impl(args.context_fn.as_ref());
    let meta_impl = args
        .meta
        .as_ref()
        .map(|e| quote! { #e })
        .unwrap_or(quote! { ::std::collections::HashMap::new() });

    let (has_loader, loader_factory_impl) = loader_factory_impl(name, args);
    let before_load_impl = before_load_impl(args);

    let stale_time_impl = args
        .stale_time
        .as_ref()
        .map(|e| quote! { Some(#e) })
        .unwrap_or(quote! { None });
    let gc_time_impl = args
        .gc_time
        .as_ref()
        .map(|e| quote! { Some(#e) })
        .unwrap_or(quote! { None });

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

    let component_registration = match &args.component {
        Some(comp_ty) => quote! {
            navi_router::components::register_route_component(<Self as ::navi_router::RouteDef>::name(), |_cx| {
                // Route components are `IntoElement` render-once values; the
                // old `gpui::Component` adapter is gone in the gpui-pre stack.
                <#comp_ty as ::gpui::IntoElement>::into_any_element(#comp_ty)
            });
        },
        None => quote! {},
    };

    quote! {
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
                    context_fn: #context_fn_impl,
                    meta: #meta_impl,
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
    }
}

pub fn define_route(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as RouteDefInput);
    let name = input.name;
    match parse_route_args(&name, input.fields).map(|args| expand(&name, &args)) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
