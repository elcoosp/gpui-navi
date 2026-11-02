use gpui::{IntoElement, RenderOnce, Window, App, ParentElement, div};
use navi_macros::define_route;
use navi_router::{Blocker, RouterHandle};
use std::sync::atomic::{AtomicBool, Ordering};

static BLOCKER_ADDED: AtomicBool = AtomicBool::new(false);

define_route!(
    BlockingRoute,
    path: "/blocking",
    component: BlockingPage,
);

#[derive(Clone, IntoElement)]
struct BlockingPage;

impl RenderOnce for BlockingPage {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        if !BLOCKER_ADDED.swap(true, Ordering::SeqCst) {
            let blocker = Blocker::new_sync(|_, _| true);
            let handle = RouterHandle::current();
            handle.update(|state| {
                state.add_blocker(blocker);
            });
            handle.refresh();
        }

        div()
            .child("This page blocks navigation")
            .child("Try navigating away - you'll be blocked")
    }
}
