use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

pub fn use_params(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = ::navi_router::RouterHandle::current();
            let current_match = state.read().current_match.clone()
                .expect("use_params called but no route matched");
            let params_map = &current_match.0;
            let json_value = serde_json::to_value(params_map)
                .expect("Failed to convert params to JSON");
            let typed_params: <#route_ty as ::navi_router::RouteDef>::Params =
                serde_json::from_value(json_value)
                    .expect("Failed to deserialize route params");
            typed_params
        }
    };
    expanded.into()
}

pub fn use_search(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = ::navi_router::RouterHandle::current();
            let location = state.read().current_location();
            let search_value = &location.search;
            match serde_json::from_value::<<#route_ty as ::navi_router::RouteDef>::Search>(search_value.clone()) {
                Ok(typed) => typed,
                Err(e) => {
                    log::warn!("Failed to deserialize search params for {}: {}, using default", stringify!(#route_ty), e);
                    Default::default()
                }
            }
        }
    };
    expanded.into()
}

pub fn use_loader_data(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = ::navi_router::RouterHandle::current();
            state.read().get_loader_data::<#route_ty>()
        }
    };
    expanded.into()
}

pub fn use_navigate(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        ::navi_router::Navigator::new()
    };
    expanded.into()
}

pub fn use_blocker(input: TokenStream) -> TokenStream {
    let should_block = parse_macro_input!(input as syn::Expr);
    let expanded = quote! {{
        let blocker = ::navi_router::Blocker::new(#should_block);
        blocker
    }};
    expanded.into()
}

pub fn use_can_go_back(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        ::navi_router::Navigator::can_go_back()
    };
    expanded.into()
}

pub fn use_match(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        {
            let state = ::navi_router::RouterHandle::current();
            state.read().current_match.clone()
                .map(|(params, node)| (node.id.clone(), params))
                .unwrap_or_default()
        }
    };
    expanded.into()
}

pub fn use_loader_state(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = ::navi_router::RouterHandle::current();
            state.read().get_loader_state::<#route_ty>()
        }
    };
    expanded.into()
}

pub fn use_route_context(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = ::navi_router::RouterHandle::current();
            state.read().get_route_context::<#route_ty>()
        }
    };
    expanded.into()
}

pub fn use_matched_route(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        {
            let state = ::navi_router::RouterHandle::current();
            let current_match = state.read().current_match.clone()
                .expect("use_matched_route called but no route matched");
            let params_map = &current_match.0;
            let node = &current_match.1;
            (params_map.clone(), node.clone())
        }
    };
    expanded.into()
}
