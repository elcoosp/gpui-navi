use gpui::prelude::*;
use gpui::{AppContext, Window, App, div, MouseButton};
use navi_macros::define_route;
use navi_router::{Blocker, RouterHandle};

#[derive(Clone, IntoElement)]
struct SettingsPage;

impl RenderOnce for SettingsPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let block_navigation = cx.new(|_| BlockerState { block: false });

        div()
            .child("⚙️ Settings Page")
            .child("Configure your application here.")
            .child(
                div().flex().gap_2().child("Block navigation:").child(
                    div()
                        .cursor_pointer()
                        .child(if block_navigation.read(cx).block {
                            "✅"
                        } else {
                            "⬜"
                        })
                        .on_mouse_up(MouseButton::Left, {
                            let block_navigation = block_navigation.clone();
                            move |_event, _window, cx| {
                                block_navigation.update(cx, |state, cx| {
                                    state.block = !state.block;
                                    cx.notify();
                                });
                            }
                        }),
                ),
            )
            .child("When checked, navigating away will be blocked.")
    }
}

struct BlockerState {
    block: bool,
}

impl Render for BlockerState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.block {
            let handle = RouterHandle::current();
            handle.update(|state| {
                state.add_blocker(Blocker::new_sync(move |_from, _to| true));
            });
            handle.refresh();
        }
        div()
    }
}

define_route!(
    SettingsRoute,
    path: "/settings",
    component: SettingsPage,
);
