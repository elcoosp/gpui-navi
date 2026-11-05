//! Index route: welcome page rendered as a `quoin_ui::Node` tree.

use navi_macros::define_route;
use navi_router::components::Link;
use quoin_ui::{Node, StyleCmd};

fn index_node() -> Node {
    Node::div()
        .styles([
            StyleCmd::FlexCol,
            StyleCmd::Gap(6.0),
            StyleCmd::Padding(12.0),
        ])
        .child(Node::text("Welcome to Navi!"))
        .child(Node::text(
            "This demo ships one shared route table that runs on gpui, dioxus, leptos, xilem and floem.",
        ))
        .child(
            Link::new("/users/42")
                .child("Open a dynamic route: /users/42")
                .build(),
        )
        .build()
}

define_route!(IndexRoute, path: "/", is_index: true, node: index_node);
