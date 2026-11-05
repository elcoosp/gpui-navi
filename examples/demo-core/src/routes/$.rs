//! Global splat (404) route.

use navi_macros::define_route;
use quoin_ui::{Node, StyleCmd};

fn not_found_node() -> Node {
    Node::div()
        .styles([StyleCmd::FlexCol, StyleCmd::Gap(6.0), StyleCmd::Padding(12.0)])
        .child(Node::text("404 — not found"))
        .child(Node::text(
            "The splat route (`$.rs`) caught this URL. Fuzzy not-found mode renders the root layout around it.",
        ))
        .build()
}

define_route!(
    GlobalNotFoundRoute,
    path: "$",
    node: not_found_node,
);
