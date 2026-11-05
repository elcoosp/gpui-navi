//! Navi demo core — one framework-agnostic demo shared by every example app.
//!
//! This crate contains **everything that is not host boilerplate**:
//!
//! - [`route_tree`] — the file-based route table (`src/routes`), generated
//!   at build time by `navi-codegen`. Route UI is expressed as
//!   `quoin_ui::Node` trees through `define_route!`'s `node:` argument, so
//!   the same routes work on any host with a quoin-ui adapter;
//! - [`driver`] — the local task driver that parks the router's `!Send`
//!   futures and polls them once per render (see the module docs);
//! - [`host`] — router installation + the per-framework cross-thread
//!   refresh seam;
//! - [`shell`] — the whole demo UI as a single quoin `component!`
//!   (`NaviDemo`). The macro emitter lowers the exact same source to GPUI,
//!   Dioxus, Leptos, Xilem and Floem, selected by this crate's framework
//!   features (`gpui`, `dioxus`, `leptos`, `xilem`, `floem`).
//!
//! The host applications under `examples/demo-*` are one-to-three-liners
//! that boot their framework with `NaviDemo`.
//!
//! ```toml
//! [dependencies]
//! navi-demo-core = { path = "../demo-core", features = ["dioxus"] }
//! ```

pub mod driver;
pub mod host;

mod route_tree {
    //! Generated route tree (see `navi.config.json` + `build.rs`).
    include!("route_tree.gen.rs");
}

#[cfg(any(
    feature = "gpui",
    feature = "dioxus",
    feature = "leptos",
    feature = "xilem",
    feature = "floem"
))]
pub mod shell;
#[cfg(any(
    feature = "gpui",
    feature = "dioxus",
    feature = "leptos",
    feature = "xilem",
    feature = "floem"
))]
pub use shell::NaviDemo;

use std::collections::HashMap;

use navi_router::{Navigator, RouterHandle};

/// Backend the shell was compiled for (surfaced in the demo header).
pub const FRAMEWORK: &str = {
    #[cfg(feature = "gpui")]
    {
        "gpui"
    }
    #[cfg(feature = "dioxus")]
    {
        "dioxus"
    }
    #[cfg(feature = "leptos")]
    {
        "leptos"
    }
    #[cfg(feature = "xilem")]
    {
        "xilem"
    }
    #[cfg(feature = "floem")]
    {
        "floem"
    }
    #[cfg(not(any(
        feature = "gpui",
        feature = "dioxus",
        feature = "leptos",
        feature = "xilem",
        feature = "floem"
    )))]
    {
        "headless"
    }
};

/// Snapshot of the router state for the shell's render pass.
///
/// Built once per render on the UI thread; plain data so the render block
/// stays simple.
#[derive(Clone, Debug)]
pub struct RouteInfo {
    /// Matched leaf route id (`RouteDef::name`), or `"404"`.
    pub route_id: String,
    /// Current pathname.
    pub pathname: String,
    /// Re-serialized query string (`k=v&…`, empty when none).
    pub query: String,
    /// Whether a pending navigation is being resolved.
    pub pending: bool,
    /// Loader state summary for the demo's loader routes.
    pub loader: &'static str,
    /// Resolved `/users/$id` loader data, when ready.
    pub user: Option<String>,
    /// Resolved `/slow` loader data, when ready.
    pub slow: Option<String>,
}

fn query_string(search: &serde_json::Value) -> String {
    match search {
        serde_json::Value::Object(map) => {
            let mut out = String::new();
            for (key, value) in map {
                if !out.is_empty() {
                    out.push('&');
                }
                out.push_str(key);
                out.push('=');
                match value {
                    serde_json::Value::String(s) => out.push_str(s),
                    serde_json::Value::Array(items) => {
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                out.push(',');
                            }
                            if let Some(s) = item.as_str() {
                                out.push_str(s);
                            } else {
                                out.push_str(&item.to_string());
                            }
                        }
                    }
                    other => out.push_str(&other.to_string()),
                }
            }
            out
        }
        _ => String::new(),
    }
}

/// Reads the current match, location and loader states from the router.
#[must_use]
pub fn read_route_info() -> RouteInfo {
    let Some(handle) = RouterHandle::try_current() else {
        return RouteInfo {
            route_id: "(router booting)".to_string(),
            pathname: "/".to_string(),
            query: String::new(),
            pending: false,
            loader: "Idle",
            user: None,
            slow: None,
        };
    };
    let state = handle.read();
    let location = state.current_location();
    let route_id = state
        .current_match
        .as_ref()
        .map_or_else(|| "404".to_string(), |(_, node)| node.id.clone());

    let slow_state = state.get_loader_state::<route_tree::slow::SlowRoute>();
    let user_state = state.get_loader_state::<route_tree::users_param_id::UsersParamIdRoute>();
    let user = state
        .get_loader_data::<route_tree::users_param_id::UsersParamIdRoute>()
        .map(|data| format!("{} — id {}", data.name, data.id));
    let slow = state
        .get_loader_data::<route_tree::slow::SlowRoute>()
        .map(|data| format!("Resolved after {} ms", data.elapsed_ms));

    RouteInfo {
        route_id,
        pathname: location.pathname.clone(),
        query: query_string(&location.search),
        pending: state.pending_navigation.is_some(),
        loader: match (user_state, slow_state) {
            (LoaderState::Loading, _) | (_, LoaderState::Loading) => "Loading",
            (LoaderState::Ready, _) | (_, LoaderState::Ready) => "Ready",
            (LoaderState::Error(_), _) | (_, LoaderState::Error(_)) => "Error",
            _ => "Idle",
        },
        user,
        slow,
    }
}

use navi_router::LoaderState;

/// Programmatic navigation (`Navigator::push`).
pub fn navigate_to(path: &str) {
    let Some(handle) = RouterHandle::try_current() else {
        log::warn!("navigate_to({path}): router not installed");
        return;
    };
    Navigator::from_handle(&handle).push(path);
}

/// Navigates to `/search?q=<query>` (or bare `/search` when empty).
pub fn navigate_search(query: String) {
    let path = if query.is_empty() {
        "/search".to_string()
    } else {
        format!("/search?q={query}")
    };
    navigate_to(&path);
}

/// History back.
pub fn go_back() {
    if let Some(handle) = RouterHandle::try_current() {
        Navigator::from_handle(&handle).back();
    }
}

/// History forward.
pub fn go_forward() {
    if let Some(handle) = RouterHandle::try_current() {
        Navigator::from_handle(&handle).forward();
    }
}

/// Exposes the generated route tree builders to hosts that want to install
/// the router themselves (the shell does it in `on_mount`).
pub fn build_route_tree() -> navi_router::RouteTree {
    route_tree::build_route_tree()
}

/// Exposes the generated component/loader registration.
pub fn register_routes() {
    route_tree::register_routes();
}

/// Unused-params-map silencer shared by route modules that don't consume
/// the raw `HashMap` form of their params.
#[allow(unused)]
pub fn _unused_params_map(_: &HashMap<String, String>) {}
