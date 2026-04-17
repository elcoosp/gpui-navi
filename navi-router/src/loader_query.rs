//! Bridge between Navi loaders and rs-query queries.

use crate::RouteNode;
use crate::loader::LoaderFn;
use gpui::App;
use rs_query::{Query, QueryError, QueryKey, QueryOptions};
use std::sync::Arc;
use std::time::Duration;

/// Create an rs-query Query from a Navi loader function.
/// The query returns `Arc<dyn Any + Send + Sync>` which can later be downcast.
pub fn create_loader_query(
    route_id: &str,
    params_json: &str,
    node: &RouteNode,
    loader_fn: LoaderFn,
    cx: &mut App,
) -> Query<Arc<dyn std::any::Any + Send + Sync>> {
    let key = format!("{}:{}", route_id, params_json);
    let query_key = QueryKey::new(&key);
    let stale_time = node.loader_stale_time.unwrap_or(Duration::ZERO);
    let gc_time = node.loader_gc_time.unwrap_or(Duration::from_secs(300));
    let options = QueryOptions {
        stale_time,
        gc_time,
        ..Default::default()
    };
    let params_map: std::collections::HashMap<String, String> =
        serde_json::from_str(params_json).unwrap_or_default();
    let executor = cx.background_executor().clone();

    let fetch_fn = move || {
        let loader_fn = loader_fn.clone();
        let params = params_map.clone();
        let exec = executor.clone();
        async move {
            // Note: We pass a dummy App context; in real usage we would need proper cx.
            // This is a simplification for the integration.
            loader_fn(&params, exec, &mut App::default()).await
        }
    };

    Query::new(query_key, fetch_fn).options(options)
}
