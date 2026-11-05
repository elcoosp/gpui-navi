//! Dynamic route with a loader: `/users/$id`.
//!
//! The loader sleeps briefly through quoin's portable timer, so the same
//! future is `Send + Sync` and resolves on every framework — the suspense
//! UI (shell `Loading…` / `Awaited` fallback) shows during the fetch.

use navi_macros::define_route;
use navi_router::components::{Awaited, Link};
use quoin_ui::{Node, StyleCmd};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Path parameters for [`UsersParamIdRoute`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserParams {
    /// The `$id` path segment.
    pub id: u32,
}

/// Data produced by the route loader.
#[derive(Clone, Debug)]
pub struct UserData {
    /// Echoed route id.
    pub id: u32,
    /// Display name resolved "from the network".
    pub name: String,
}

async fn load_user(params: UserParams) -> Result<UserData, String> {
    // Stand-in for network I/O; quoin's portable sleep keeps this future
    // framework-agnostic and wasm-safe.
    navi_router::sleep(Duration::from_millis(250)).await;
    Ok(UserData {
        id: params.id,
        name: format!("User #{}", params.id),
    })
}

fn user_node() -> Node {
    Node::div()
        .styles([StyleCmd::FlexCol, StyleCmd::Gap(6.0), StyleCmd::Padding(12.0)])
        .child(Node::text("User profile (node route with loader)"))
        .child(
            Awaited::<UsersParamIdRoute>::new()
                .fallback(|| Node::text("Loading user…"))
                .child(|data: UserData| {
                    Node::text(format!("{} — id {}", data.name, data.id))
                })
                .build(),
        )
        .child(Link::new("/users").child("Back to users index").build())
        .build()
}

define_route!(
    UsersParamIdRoute,
    path: "/users/$id",
    params: UserParams,
    data: UserData,
    loader: |params: UserParams| async move { load_user(params).await },
    stale_time: Duration::from_secs(30),
    node: user_node,
);
