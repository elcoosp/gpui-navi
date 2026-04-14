use gpui::{App, Div, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px, rgb};
use navi_router::{RouterEvent, RouterState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevtoolsTab {
    Routes,
    Cache,
    Timeline,
    State,
}

pub struct NaviDevtools {
    expanded: bool,
    selected_tab: DevtoolsTab,
    event_log: Vec<crate::timeline::LoggedEvent>,
}

impl NaviDevtools {
    pub fn new() -> Self {
        Self {
            expanded: true,
            selected_tab: DevtoolsTab::Routes,
            event_log: Vec::new(),
        }
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn selected_tab(mut self, tab: DevtoolsTab) -> Self {
        self.selected_tab = tab;
        self
    }

    pub fn add_event(&mut self, event: RouterEvent) {
        self.event_log
            .push(crate::timeline::LoggedEvent::new(event));
        if self.event_log.len() > 100 {
            self.event_log.remove(0);
        }
    }

    fn render_tab_button(&self, tab: DevtoolsTab, label: &str) -> Div {
        let selected = self.selected_tab == tab;
        div()
            .px_2()
            .py_1()
            .cursor_pointer()
            .bg(if selected {
                rgb(0x3a3a3a)
            } else {
                rgb(0x2a2a2a)
            })
            .text_color(if selected {
                rgb(0xffffff)
            } else {
                rgb(0xaaaaaa)
            })
            .child(label.to_string())
    }

    fn render_routes_tab(&self, cx: &mut App) -> Div {
        let state = RouterState::try_global(cx);
        let mut container = div().p_2().gap_1().flex().flex_col();

        if let Some(state) = state {
            container = container.child("Route Tree:");
            for node in state.route_tree.all_nodes() {
                let is_active = state
                    .current_match
                    .as_ref()
                    .map(|(_, n)| n.id == node.id)
                    .unwrap_or(false);
                let indent = node.parent.as_ref().map(|_| "  ").unwrap_or("");
                container = container.child(
                    div()
                        .child(format!(
                            "{}{} {}",
                            indent,
                            if is_active { "→" } else { " " },
                            node.id
                        ))
                        .text_color(if is_active {
                            rgb(0x4ec9b0)
                        } else {
                            rgb(0xd4d4d4)
                        }),
                );
            }
            if let Some((params, _)) = &state.current_match {
                container = container.child("Params:").child(format!("{:?}", params));
            }
        } else {
            container = container.child("No router state found");
        }
        container
    }

    fn render_timeline_tab(&self) -> Div {
        let mut container = div().p_2().gap_1().flex().flex_col().text_sm();
        for event in self.event_log.iter().rev() {
            container = container.child(format!(
                "[{}] {:?}",
                event.timestamp.format("%H:%M:%S%.3f"),
                event.event
            ));
        }
        if self.event_log.is_empty() {
            container = container.child("No events yet");
        }
        container
    }

    fn render_cache_tab(&self, _cx: &mut App) -> Div {
        div().p_2().child("Cache inspection (rs-query integration)")
    }

    fn render_state_tab(&self, cx: &mut App) -> Div {
        let state = RouterState::try_global(cx);
        let mut container = div().p_2().gap_2().flex().flex_col();
        if let Some(state) = state {
            container = container
                .child("Current Location:")
                .child(format!("  {}", state.current_location().pathname))
                .child("History:")
                .child(format!("  can_go_back: {}", state.history.can_go_back()))
                .child(format!(
                    "  can_go_forward: {}",
                    state.history.can_go_forward()
                ))
                .child("Blockers:")
                .child(format!("  count: {}", state.blockers.len()));
        } else {
            container = container.child("No router state");
        }
        container
    }
}

impl Default for NaviDevtools {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for NaviDevtools {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.expanded {
            return div();
        }

        div()
            .absolute()
            .bottom_0()
            .right_0()
            .w(px(400.0))
            .h(px(300.0))
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xd4d4d4))
            .border_1()
            .border_color(rgb(0x3a3a3a))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .border_b_1()
                    .border_color(rgb(0x3a3a3a))
                    .child(self.render_tab_button(DevtoolsTab::Routes, "Routes"))
                    .child(self.render_tab_button(DevtoolsTab::Cache, "Cache"))
                    .child(self.render_tab_button(DevtoolsTab::Timeline, "Timeline"))
                    .child(self.render_tab_button(DevtoolsTab::State, "State")),
            )
            .child(div().flex_1().child(match self.selected_tab {
                DevtoolsTab::Routes => self.render_routes_tab(cx).into_any_element(),
                DevtoolsTab::Cache => self.render_cache_tab(cx).into_any_element(),
                DevtoolsTab::Timeline => self.render_timeline_tab().into_any_element(),
                DevtoolsTab::State => self.render_state_tab(cx).into_any_element(),
            }))
    }
}
