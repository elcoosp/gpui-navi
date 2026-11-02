//! Scroll restoration: policy closures over router state + a shared
//! adapter scroll handle.

use crate::RouterHandle;
use quoin_ui::Node;

/// Builds a scroll-restoration node bound to the adapter's shared scroll
/// handle `handle_id` (the page attaches the same handle to its scroll
/// container via the adapter's `shared_scroll_handle` helper).
#[must_use]
pub fn scroll_restoration_node(handle_id: impl Into<String>) -> Node {
    let Some(handle) = RouterHandle::try_current() else {
        return Node::div().build();
    };
    quoin_ui::scroll_restoration(handle_id, Node::div().build())
        .current_key({
            let handle = handle.clone();
            move || {
                let state = handle.read();
                if !state.scroll_restoration {
                    return None;
                }
                let loc = state.current_location();
                Some(crate::state::ScrollPositions::key(
                    &loc.pathname,
                    &loc.search,
                ))
            }
        })
        .saved_offset({
            let handle = handle.clone();
            move |key| handle.read().scroll_positions.get(key)
        })
        .save_offset({
            let handle = handle.clone();
            move |key, offset| {
                handle
                    .read()
                    .scroll_positions
                    .insert(key.to_string(), offset)
            }
        })
        .skip_once({
            let handle = handle.clone();
            move || handle.write().take_scroll_reset()
        })
        .build()
}

/// The scroll-restoration builder (classic name kept for parity).
pub struct ScrollRestoration;

impl ScrollRestoration {
    /// Builds the restoration node for `handle_id`.
    #[must_use]
    pub fn node(handle_id: impl Into<String>) -> Node {
        scroll_restoration_node(handle_id)
    }

    /// Bound how many locations remember their scroll offset (oldest
    /// evicted first). Defaults to
    /// [`DEFAULT_SCROLL_CACHE_CAPACITY`](crate::state::DEFAULT_SCROLL_CACHE_CAPACITY).
    pub fn set_capacity(capacity: usize) {
        if let Some(handle) = RouterHandle::try_current() {
            handle.read().scroll_positions.set_capacity(capacity);
        }
    }
}
