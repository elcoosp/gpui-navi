// navi-devtools/src/deep_link_view.rs
//! Deep link viewer component (feature = "nexum").

use gpui::*;
use navi_router::deep_link::{DeepLinkEvent, DeepLinkStatus};

pub struct DeepLinkView {
    events: Vec<DeepLinkEvent>,
    receiver: Option<tokio::sync::broadcast::Receiver<DeepLinkEvent>>,
}

impl DeepLinkView {
    pub fn new() -> Self {
        let receiver = navi_router::deep_link::subscribe_events();
        Self {
            events: Vec::new(),
            receiver,
        }
    }

    fn clear(&mut self) {
        self.events.clear();
    }
}

impl Render for DeepLinkView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        if let Some(rx) = &mut self.receiver {
            while let Ok(event) = rx.try_recv() {
                self.events.push(event);
                if self.events.len() > 100 {
                    self.events.remove(0);
                }
            }
        }

        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().font_weight(FontWeight::BOLD).child("Deep Links"))
                    .child(
                        div()
                            // FIX: You MUST call .id() to make a Div stateful so it can receive clicks!
                            .id("clear-deep-links-btn")
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                this.clear();
                            }))
                            .child("Clear"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(self.events.iter().rev().map(|event| {
                        let (status_icon, status_color) = match event.status {
                            DeepLinkStatus::Success => ("✅", 0x6a9955),
                            DeepLinkStatus::Blocked => ("⏸️", 0xdcdcaa),
                            DeepLinkStatus::ParseError => ("❌", 0xf44747),
                        };
                        let time = event.timestamp.format("%H:%M:%S").to_string();
                        let route_info = event
                            .matched_route
                            .as_ref()
                            .map(|id| format!(" → {}", id))
                            .unwrap_or_default();

                        div()
                            .flex()
                            .gap_2()
                            .child(div().text_color(rgb(0x808080)).child(time))
                            .child(div().text_color(rgb(status_color)).child(status_icon))
                            .child(div().child(format!("{}{}", event.url, route_info)))
                    })),
            )
    }
}
