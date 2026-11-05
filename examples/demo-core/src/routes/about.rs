//! About route: static node content.

use navi_macros::define_route;
use quoin_ui::{Node, StyleCmd};

fn about_node() -> Node {
    Node::div()
        .styles([
            StyleCmd::FlexCol,
            StyleCmd::Gap(6.0),
            StyleCmd::Padding(12.0),
        ])
        .child(Node::text("About Navi"))
        .child(Node::text(
            "Navi is a file-based router built on quoin: one route table, many GUI frameworks.",
        ))
        .build()
}

define_route!(AboutRoute, path: "/about", node: about_node);
