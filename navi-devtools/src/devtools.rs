use gpui::{prelude::*, *};
use gpui_component::{
    Icon, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    scroll::ScrollableElement,
};
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.expanded {
            return div();
        }

        // Fixed dark theme colors
        let bg: Hsla = rgb(0x1e1e2e).into();
        let fg: Hsla = rgb(0xcdd6f4).into();
        let secondary_bg: Hsla = rgb(0x313244).into();
        let border_color: Hsla = rgb(0x45475a).into();
        let primary: Hsla = rgb(0x89b4fa).into();
        let success: Hsla = rgb(0xa6e3a1).into();
        let warning: Hsla = rgb(0xf9e2af).into();
        let info: Hsla = rgb(0x89dceb).into();
        let muted_fg: Hsla = rgb(0x9399b2).into();

        let viewport = window.viewport_size();
        let panel_width = px(550.0).min(viewport.width - px(20.0));
        let panel_height = px(450.0).min(viewport.height - px(20.0));

        div()
            .absolute()
            .bottom_0()
            .right_0()
            .w(panel_width)
            .h(panel_height)
            .bg(bg)
            .text_color(fg)
            .border_1()
            .border_color(border_color)
            .rounded_tl(px(8.0))
            .shadow_lg()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .bg(secondary_bg)
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(Icon::new(IconName::Info))
                            .child(" Devtools"),
                    )
                    .child(
                        Button::new("close-devtools")
                            .ghost()
                            .xsmall()
                            .icon(Icon::new(IconName::Close))
                            .on_click(cx.listener(
                                |this: &mut Self, _event: &gpui::ClickEvent, _window, cx| {
                                    this.toggle_expanded(cx);
                                },
                            )),
                    ),
            )
            .child(
                // Custom tab bar
                div()
                    .flex()
                    .border_b_1()
                    .border_color(border_color)
                    .child(self.render_tab_button(
                        DevtoolsTab::Routes,
                        "Routes",
                        IconName::Folder,
                        primary,
                        fg,
                        cx,
                    ))
                    .child(self.render_tab_button(
                        DevtoolsTab::Cache,
                        "Cache",
                        IconName::Inbox,
                        primary,
                        fg,
                        cx,
                    ))
                    .child(self.render_tab_button(
                        DevtoolsTab::Timeline,
                        "Timeline",
                        IconName::Calendar,
                        primary,
                        fg,
                        cx,
                    ))
                    .child(self.render_tab_button(
                        DevtoolsTab::State,
                        "State",
                        IconName::Settings,
                        primary,
                        fg,
                        cx,
                    )),
            )
            .child(
                // Scrollable content area
                div()
                    .flex_1()
                    .p_3()
                    .pb_6()
                    .child(match self.selected_tab {
                        DevtoolsTab::Routes => self.render_routes_tab(
                            fg,
                            secondary_bg,
                            border_color,
                            primary,
                            success,
                            warning,
                            info,
                            cx,
                        ),
                        DevtoolsTab::Cache => self.render_cache_tab(info, muted_fg),
                        DevtoolsTab::Timeline => {
                            self.render_timeline_tab(fg, border_color, info, muted_fg)
                        }
                        DevtoolsTab::State => {
                            self.render_state_tab(fg, secondary_bg, border_color, info, warning, cx)
                        }
                    })
                    .overflow_y_scrollbar(),
            )
    }
}

impl DevtoolsState {
    fn render_tab_button(
        &self,
        tab: DevtoolsTab,
        label: &'static str,
        icon: IconName,
        primary: Hsla,
        fg: Hsla,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self.selected_tab == tab;
        div()
            .px_3()
            .py_1()
            .cursor_pointer()
            .bg(if is_selected {
                primary.opacity(0.15)
            } else {
                Hsla::transparent_black()
            })
            .border_b_2()
            .border_color(if is_selected {
                primary
            } else {
                Hsla::transparent_black()
            })
            .hover(|style| {
                style.bg(if is_selected {
                    primary.opacity(0.2)
                } else {
                    fg.opacity(0.1)
                })
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_color(if is_selected { primary } else { fg })
                    .child(Icon::new(icon))
                    .child(label),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this: &mut Self, _event, _window, cx| {
                    this.set_selected_tab(tab, cx);
                }),
            )
    }

    fn render_routes_tab(
        &self,
        _fg: Hsla,
        secondary_bg: Hsla,
        border_color: Hsla,
        primary: Hsla,
        success: Hsla,
        warning: Hsla,
        info: Hsla,
        cx: &mut Context<Self>,
    ) -> Div {
        let state = RouterState::try_global(cx);
        let mut container = div().gap_2().flex().flex_col();

        if let Some(state) = state {
            let current_location = state.current_location();

            container = container.child(
                div()
                    .p_3()
                    .bg(secondary_bg)
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .text_color(primary)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(Icon::new(IconName::Map))
                                    .child("Current Location"),
                            )
                            .child(format!("Path: {}", current_location.pathname))
                            .child(format!("Search: {:?}", current_location.search)),
                    ),
            );

            container = container.child(
                div()
                    .p_3()
                    .bg(secondary_bg)
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(border_color)
                    .child({
                        let mut card = div().flex().flex_col().gap_1().child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_color(success)
                                .font_weight(FontWeight::MEDIUM)
                                .child(Icon::new(IconName::Check))
                                .child("Matched Route"),
                        );

                        if let Some((params, node)) = &state.current_match {
                            card = card
                                .child(format!("ID: {}", node.id))
                                .child(format!("Pattern: {}", node.pattern.raw))
                                .child("Params:")
                                .child(format!("{:?}", params));
                        } else {
                            card = card.child("No route matched!").text_color(warning);
                        }
                        card
                    }),
            );

            container = container.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_color(info)
                    .font_weight(FontWeight::MEDIUM)
                    .child(Icon::new(IconName::FolderOpen))
                    .child("Route Tree"),
            );

            for node in state.route_tree.all_nodes() {
                let is_active = state
                    .current_match
                    .as_ref()
                    .map(|(_, n)| n.id == node.id)
                    .unwrap_or(false);
                let indent = node.parent.as_ref().map(|_| "  ").unwrap_or("");
                let marker = if is_active { "→" } else { " " };
                let text = format!("{}{} {} ({})", indent, marker, node.id, node.pattern.raw);
                container = container.child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded(px(4.0))
                        .when(is_active, |this: Div| {
                            this.bg(primary.opacity(0.2)).text_color(primary)
                        })
                        .child(text),
                );
            }
        } else {
            container = container.child(div().text_color(warning).child("No router state found"));
        }
        container
    }

    fn render_timeline_tab(&self, fg: Hsla, border_color: Hsla, info: Hsla, muted_fg: Hsla) -> Div {
        div()
            .gap_2()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_color(info)
                    .font_weight(FontWeight::MEDIUM)
                    .child(Icon::new(IconName::Calendar))
                    .child("Event Timeline"),
            )
            .children(self.event_log.iter().rev().map(|event| {
                div()
                    .px_2()
                    .py_1()
                    .text_sm()
                    .border_b_1()
                    .border_color(border_color.opacity(0.5))
                    .text_color(fg)
                    .child(format!(
                        "[{}] {:?}",
                        event.timestamp.format("%H:%M:%S%.3f"),
                        event.event
                    ))
            }))
            .when(self.event_log.is_empty(), |this: Div| {
                this.child(div().text_color(muted_fg).child("No events yet"))
            })
    }

    fn render_cache_tab(&self, info: Hsla, muted_fg: Hsla) -> Div {
        div()
            .gap_2()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_color(info)
                    .font_weight(FontWeight::MEDIUM)
                    .child(Icon::new(IconName::Inbox))
                    .child("Cache Inspection"),
            )
            .child(
                div()
                    .text_color(muted_fg)
                    .child("Cache inspection (rs-query integration)"),
            )
    }

    fn render_state_tab(
        &self,
        fg: Hsla,
        secondary_bg: Hsla,
        border_color: Hsla,
        info: Hsla,
        warning: Hsla,
        cx: &mut Context<Self>,
    ) -> Div {
        let state = RouterState::try_global(cx);
        let mut container = div().gap_2().flex().flex_col();

        if let Some(state) = state {
            container = container
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(info)
                        .font_weight(FontWeight::MEDIUM)
                        .child(Icon::new(IconName::Info))
                        .child("Router State"),
                )
                .child(
                    div()
                        .p_3()
                        .bg(secondary_bg)
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(border_color)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .text_color(fg)
                                .child(format!(
                                    "Current Location: {}",
                                    state.current_location().pathname
                                ))
                                .child(format!("Can go back: {}", state.history.can_go_back()))
                                .child(format!(
                                    "Can go forward: {}",
                                    state.history.can_go_forward()
                                ))
                                .child(format!("Blockers count: {}", state.blockers.len())),
                        ),
                );
        } else {
            container = container.child(div().text_color(warning).child("No router state"));
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
