//! Demo root layout: a node-based header (real navi `Link`s) around the
//! matched child route.
//!
//! The route UI here is built from `quoin_ui::Node` trees, so it is fully
//! framework-agnostic: any host with a quoin-ui adapter renders it as-is
//! (the GPUI adapter ships today). The `NaviDemo` shell in this crate
//! mirrors the same routes with quoin `component!` UI for hosts that prefer
//! the macro path.

use navi_macros::define_route;
use navi_router::components::{Link, Outlet};
use quoin_ui::{Node, StyleCmd};

/// Header row: one navi `Link` per demo destination.
fn link_bar() -> Node {
    let destinations = [
        ("/", "Home"),
        ("/about", "About"),
        ("/users/1", "User 1"),
        ("/slow", "Slow loader"),
        ("/guarded", "Guarded"),
        ("/search", "Search"),
        ("/nope", "404"),
    ];
    let mut bar = Node::div()
        .styles([StyleCmd::Flex, StyleCmd::FlexWrap, StyleCmd::Gap(12.0)]);
    for (href, label) in destinations {
        let link = Link::new(href).child(label).build();
        bar = bar.child(link);
    }
    bar.build()
}

/// The whole layout: title, links, then the matched child route.
fn root_node() -> Node {
    Node::div()
        .styles([
            StyleCmd::SizeFull,
            StyleCmd::FlexCol,
            StyleCmd::Gap(8.0),
            StyleCmd::Padding(8.0),
        ])
        .child(Node::text("Navi demo — node routes"))
        .child(link_bar())
        .child(Outlet::new().build())
        .build()
}

define_route!(RootRoute, path: "/", is_layout: true, node: root_node);
