//! Search-params route: typed query parsing rendered from the router state.

use navi_macros::define_route;
use quoin_ui::{Node, StyleCmd};
use serde::{Deserialize, Serialize};

/// No path parameters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchParams {}

fn search_node() -> Node {
    Node::div()
        .styles([StyleCmd::FlexCol, StyleCmd::Gap(6.0), StyleCmd::Padding(12.0)])
        .child(Node::text("Search params"))
        .child(Node::text(
            "Try /search?q=navi&page=2 — the shell mirrors the parsed query for this route.",
        ))
        .build()
}

define_route!(
    SearchRoute,
    path: "/search",
    params: SearchParams,
    node: search_node,
);
