use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::{Blocker, RouterState};

#[derive(Clone, IntoElement)]
struct SettingsPage;

impl RenderOnce for SettingsPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let block_navigation = cx.new(|cx| {
            let focus_handle = cx.focus_handle();
            focus_handle.focus(window, cx);
            BlockerState {
                block: false,
                focus_handle,
            }
        });

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
    #[allow(dead_code)]
    focus_handle: FocusHandle,
}

impl Render for BlockerState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.block {
            RouterState::update(cx, |state, _| {
                state.add_blocker(Blocker::new(move |_from, _to| true));
            });
        }
        div()
    }
}

define_route!(
    SettingsRoute,
    path: "/settings",
    component: SettingsPage,
);
