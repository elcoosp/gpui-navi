//! Slow loader route: a 900 ms fetch, showing the pending/suspense window.

use navi_macros::define_route;
use navi_router::components::Awaited;
use quoin_ui::{Node, StyleCmd};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// No path parameters.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SlowParams {}

/// Data produced by the slow loader.
#[derive(Clone, Debug)]
pub struct SlowData {
    /// Wall-clock elapsed time, resolved "from the network".
    pub elapsed_ms: u64,
}

async fn load_slow(_params: SlowParams) -> Result<SlowData, String> {
    let start = std::time::Instant::now();
    navi_router::sleep(Duration::from_millis(900)).await;
    Ok(SlowData {
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

fn slow_node() -> Node {
    Node::div()
        .styles([StyleCmd::FlexCol, StyleCmd::Gap(6.0), StyleCmd::Padding(12.0)])
        .child(Node::text("Slow loader (900 ms)"))
        .child(
            Awaited::<SlowRoute>::new()
                .fallback(|| Node::text("Loading… (watch the pending window)"))
                .child(|data: SlowData| {
                    Node::text(format!("Resolved after {} ms", data.elapsed_ms))
                })
                .build(),
        )
        .build()
}

define_route!(
    SlowRoute,
    path: "/slow",
    params: SlowParams,
    data: SlowData,
    loader: |params: SlowParams| async move { load_slow(params).await },
    node: slow_node,
);
