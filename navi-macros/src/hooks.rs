// navi-macros/src/hooks.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

pub fn use_params(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            let state = navi_router::RouterState::global(cx);
            let current_match = state.current_match.as_ref()
                .expect("use_params called but no route matched");
            let params_map = &current_match.0;
            let json_value = serde_json::to_value(params_map)
                .expect("Failed to convert params to JSON");
            let typed_params: <#route_ty as navi_router::RouteDef>::Params =
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
            let state = navi_router::RouterState::global(cx);
            let location = state.current_location();
            let search_value = &location.search;
            match serde_json::from_value::<<#route_ty as navi_router::RouteDef>::Search>(search_value.clone()) {
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
    // No longer triggers loader — that's done in navigate()
    let expanded = quote! {
        {
            navi_router::RouterState::try_global(cx)
                .and_then(|s| s.get_loader_data::<#route_ty>())
        }
    };
    expanded.into()
}

pub fn use_navigate(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        {
            navi_router::Navigator::new(window.window_handle())
        }
    };
    expanded.into()
}

pub fn use_blocker(input: TokenStream) -> TokenStream {
    let should_block = parse_macro_input!(input as syn::Expr);
    let expanded = quote! {{
        let blocker = navi_router::Blocker::new(#should_block);
        blocker
    }};
    expanded.into()
}

pub fn use_can_go_back(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        navi_router::RouterState::try_global(cx)
            .map(|s| s.history.can_go_back())
            .unwrap_or(false)
    };
    expanded.into()
}
