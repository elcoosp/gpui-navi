use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::{Blocker, RouterState};

define_route!(
    BlockingRoute,
    path: "/blocking",
    component: BlockingPage,
);

#[derive(Clone, IntoElement)]
struct BlockingPage;

impl RenderOnce for BlockingPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let blocker = Blocker::new(|_from, _to| async move {
            // Simulate async confirmation - block navigation
            true
        });
        RouterState::update(cx, |state, _| {
            state.add_blocker(blocker);
        });
        div()
            .child("This page blocks navigation (async)")
            .child("Try navigating away - you'll be blocked")
    }
}
