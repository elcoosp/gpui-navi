//! Guarded route: `before_load` redirects to `/` unless the `key` search
//! param carries the passphrase.

use navi_macros::define_route;
use navi_router::route_tree::{BeforeLoadContext, BeforeLoadResult};
use quoin_ui::{Node, StyleCmd};
use serde::{Deserialize, Serialize};

/// No path parameters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GuardedParams {}

/// Parses the passphrase out of the parsed-query JSON (values may be plain
/// strings or single-element arrays depending on repetition).
fn passphrase_of(search: &serde_json::Value) -> Option<&str> {
    search
        .get("key")
        .and_then(|v| v.as_str().or_else(|| v.get(0).and_then(|s| s.as_str())))
}

fn guard(ctx: BeforeLoadContext) -> BeforeLoadResult {
    if passphrase_of(&ctx.search) == Some("sesame") {
        BeforeLoadResult::Ok
    } else {
        BeforeLoadResult::Redirect(navi_router::redirect("/?redirected=guarded"))
    }
}

fn guarded_node() -> Node {
    Node::div()
        .styles([StyleCmd::FlexCol, StyleCmd::Gap(6.0), StyleCmd::Padding(12.0)])
        .child(Node::text("Guarded area"))
        .child(Node::text(
            "You came in with the right key — beforeLoad let this navigation through.",
        ))
        .build()
}

define_route!(
    GuardedRoute,
    path: "/guarded",
    params: GuardedParams,
    before_load: |ctx: BeforeLoadContext| async move { guard(ctx) },
    node: guarded_node,
);
