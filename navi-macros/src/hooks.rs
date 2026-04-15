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
            let typed_search: <#route_ty as navi_router::RouteDef>::Search =
                serde_json::from_value(search_value.clone())
                    .expect("Failed to deserialize search params");
            typed_search
        }
    };
    expanded.into()
}

pub fn use_loader_data(input: TokenStream) -> TokenStream {
    let route_ty = parse_macro_input!(input as syn::Type);
    let expanded = quote! {
        {
            // Check if data is already available without holding borrow across update
            let data_exists = navi_router::RouterState::try_global(cx)
                .and_then(|s| s.get_loader_data::<#route_ty>())
                .is_some();

            if !data_exists {
                navi_router::RouterState::update(cx, |state, cx| state.trigger_loader(cx));
            }

            navi_router::RouterState::global(cx)
                .get_loader_data::<#route_ty>()
                .expect("loader data not ready")
                .clone()
        }
    };
    expanded.into()
}

pub fn use_navigate(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        {
            let window_id = cx.window_handle().window_id();
            navi_router::Navigator::new(window_id)
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
        false
    };
    expanded.into()
}
