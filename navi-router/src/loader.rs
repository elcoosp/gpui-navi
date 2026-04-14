//! Loader cache integration with rs-query for SWR caching.

use rs_query::QueryClient;

pub type LoaderError = Box<dyn std::error::Error + Send + Sync>;

/// Invalidate cached loader data matching a query key prefix.
pub fn invalidate_loader_cache(query_client: &QueryClient, key_prefix: rs_query::QueryKey) {
    query_client.invalidate_queries(&key_prefix);
}
