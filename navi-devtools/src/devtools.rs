use gpui::*;
use navi_router::{RouterEvent, RouterState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevtoolsTab {
    Routes,
    Cache,
    Timeline,
    State,
}

pub struct DevtoolsState {
    expanded: bool,
    selected_tab: DevtoolsTab,
    event_log: Vec<crate::timeline::LoggedEvent>,
}

impl EventEmitter<()> for DevtoolsState {}

impl DevtoolsState {
    pub fn new() -> Self {
        Self {
            expanded: true,
            selected_tab: DevtoolsTab::Routes,
            event_log: Vec::new(),
        }
    }

    pub fn add_event(&mut self, event: RouterEvent, cx: &mut Context<Self>) {
        self.event_log
            .push(crate::timeline::LoggedEvent::new(event));
        if self.event_log.len() > 100 {
            self.event_log.remove(0);
        }
        cx.notify();
    }

    fn set_selected_tab(&mut self, tab: DevtoolsTab, cx: &mut Context<Self>) {
        self.selected_tab = tab;
        cx.notify();
    }

    #[allow(dead_code)]
    fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        cx.notify();
    }
}

impl Render for DevtoolsState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.expanded {
            return div();
        }

        let mut content = div().flex_1();
        content.style().overflow.y = Some(Overflow::Scroll);

        div()
            .absolute()
            .bottom_0()
            .right_0()
            .w(px(500.0))
            .h(px(400.0))
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
                    .child(self.render_tab_button(DevtoolsTab::Routes, "Routes", cx))
                    .child(self.render_tab_button(DevtoolsTab::Cache, "Cache", cx))
                    .child(self.render_tab_button(DevtoolsTab::Timeline, "Timeline", cx))
                    .child(self.render_tab_button(DevtoolsTab::State, "State", cx)),
            )
            .child(content.child(match self.selected_tab {
                DevtoolsTab::Routes => self.render_routes_tab(cx).into_any_element(),
                DevtoolsTab::Cache => self.render_cache_tab().into_any_element(),
                DevtoolsTab::Timeline => self.render_timeline_tab().into_any_element(),
                DevtoolsTab::State => self.render_state_tab(cx).into_any_element(),
            }))
    }
}

impl DevtoolsState {
    fn render_tab_button(&self, tab: DevtoolsTab, label: &str, cx: &mut Context<Self>) -> Div {
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
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this: &mut Self, _event: &MouseUpEvent, _window, cx| {
                    this.set_selected_tab(tab, cx);
                }),
            )
    }

    fn render_routes_tab(&self, cx: &mut Context<Self>) -> Div {
        let state = RouterState::try_global(cx);
        let mut container = div().p_2().gap_1().flex().flex_col();

        if let Some(state) = state {
            let current_location = state.current_location();
            container = container
                .child("Current Location:")
                .child(format!("  Path: {}", current_location.pathname))
                .child(format!("  Search: {:?}", current_location.search));

            if let Some((params, node)) = &state.current_match {
                container = container
                    .child("Matched Route:")
                    .child(format!("  ID: {}", node.id))
                    .child(format!("  Pattern: {}", node.pattern.raw))
                    .child("Params:")
                    .child(format!("  {:?}", params));
            } else {
                container = container
                    .child("No route matched!")
                    .text_color(rgb(0xff5555));
            }

            container = container.child("Route Tree:").child(div().pt_2());
            for node in state.route_tree.all_nodes() {
                let is_active = state
                    .current_match
                    .as_ref()
                    .map(|(_, n)| n.id == node.id)
                    .unwrap_or(false);
                let indent = node.parent.as_ref().map(|_| "  ").unwrap_or("");
                let marker = if is_active { "→" } else { " " };
                let text = format!("{}{} {} ({})", indent, marker, node.id, node.pattern.raw);
                container = container.child(div().child(text).text_color(if is_active {
                    rgb(0x4ec9b0)
                } else {
                    rgb(0xd4d4d4)
                }));
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

    fn render_cache_tab(&self) -> Div {
        div().p_2().child("Cache inspection (rs-query integration)")
    }

    fn render_state_tab(&self, cx: &mut Context<Self>) -> Div {
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

#[derive(Clone)]
pub struct NaviDevtools {
    state: Entity<DevtoolsState>,
}

impl NaviDevtools {
    pub fn new(cx: &mut App) -> Self {
        Self {
            state: cx.new(|_cx| DevtoolsState::new()),
        }
    }
}

impl RenderOnce for NaviDevtools {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().child(self.state)
    }
}

impl IntoElement for NaviDevtools {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
