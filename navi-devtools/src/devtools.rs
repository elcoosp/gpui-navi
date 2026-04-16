use gpui::{
    App, Context, Entity, EventEmitter, FontWeight, Render, RenderOnce, Size, Subscription, Window,
    div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, VirtualListScrollHandle,
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement,
    tab::{Tab, TabBar},
    v_virtual_list,
};
use navi_router::{RouterState, event_bus};
use std::rc::Rc;

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
    timeline_search: Option<Entity<InputState>>,
    timeline_scroll_handle: VirtualListScrollHandle,
    _subscription: Subscription,
}

impl EventEmitter<()> for DevtoolsState {}

impl DevtoolsState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_global::<RouterState>(move |this, cx| {
            this.refresh_log(cx);
        });

        let mut this = Self {
            expanded: true,
            selected_tab: DevtoolsTab::Routes,
            event_log: Vec::new(),
            timeline_search: None,
            timeline_scroll_handle: VirtualListScrollHandle::new(),
            _subscription: subscription,
        };
        this.refresh_log(cx);
        this
    }

    fn ensure_timeline_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.timeline_search.is_none() {
            let state = cx.new(|cx| {
                let mut s = InputState::new(window, cx);
                s.set_placeholder("Search events...", window, cx);
                s
            });
            cx.subscribe::<InputState, InputEvent>(&state, |_, _, _event, cx| {
                cx.notify();
            })
            .detach();
            self.timeline_search = Some(state);
        }
    }

    fn refresh_log(&mut self, cx: &mut Context<Self>) {
        let events = event_bus::get_event_log(cx);
        self.event_log = events
            .into_iter()
            .map(crate::timeline::LoggedEvent::new)
            .collect();
        cx.notify();
    }

    fn filtered_events(&self, cx: &App) -> Vec<crate::timeline::LoggedEvent> {
        let query = self
            .timeline_search
            .as_ref()
            .map(|s| s.read(cx).value().to_lowercase())
            .unwrap_or_default();
        if query.is_empty() {
            self.event_log.clone()
        } else {
            self.event_log
                .iter()
                .filter(|e| {
                    let event_str = format!("{:?}", e.event).to_lowercase();
                    event_str.contains(&query)
                })
                .cloned()
                .collect()
        }
    }

    fn set_selected_tab(&mut self, tab: DevtoolsTab, cx: &mut Context<Self>) {
        self.selected_tab = tab;
        cx.notify();
    }

    fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        cx.notify();
    }

    fn tab_index(&self) -> usize {
        match self.selected_tab {
            DevtoolsTab::Routes => 0,
            DevtoolsTab::Cache => 1,
            DevtoolsTab::Timeline => 2,
            DevtoolsTab::State => 3,
        }
    }

    fn tab_from_index(&self, idx: usize) -> DevtoolsTab {
        match idx {
            0 => DevtoolsTab::Routes,
            1 => DevtoolsTab::Cache,
            2 => DevtoolsTab::Timeline,
            3 => DevtoolsTab::State,
            _ => DevtoolsTab::Routes,
        }
    }

    fn render_routes_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let state = RouterState::try_global(cx);

        let mut container = div().gap_3().flex().flex_col();

        if let Some(state) = state {
            container = container.child(
                div()
                    .p_3()
                    .bg(theme.secondary)
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.border)
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
                                    .text_color(theme.primary)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(Icon::new(IconName::Map))
                                    .child("Current Location"),
                            )
                            .child(format!("Path: {}", state.current_location().pathname))
                            .child(format!("Search: {:?}", state.current_location().search)),
                    ),
            );

            container = container.child(
                div()
                    .p_3()
                    .bg(theme.secondary)
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.border)
                    .child({
                        let mut card = div().flex().flex_col().gap_1().child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_color(theme.success)
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
                            card = card
                                .child(div().text_color(theme.warning).child("No route matched!"));
                        }
                        card
                    }),
            );

            container = container
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(theme.info)
                        .font_weight(FontWeight::MEDIUM)
                        .child(Icon::new(IconName::FolderOpen))
                        .child("Route Tree"),
                )
                .children(state.route_tree.all_nodes().map(|node| {
                    let is_active = state
                        .current_match
                        .as_ref()
                        .map(|(_, n)| n.id == node.id)
                        .unwrap_or(false);
                    let indent = node.parent.as_ref().map(|_| "  ").unwrap_or("");
                    let marker = if is_active { "→" } else { " " };
                    let text = format!("{}{} {} ({})", indent, marker, node.id, node.pattern.raw);
                    div()
                        .px_2()
                        .py_1()
                        .rounded(px(4.0))
                        .when(is_active, |d| {
                            d.bg(theme.primary.opacity(0.2)).text_color(theme.primary)
                        })
                        .child(text)
                }));
        } else {
            container = container.child(
                div()
                    .text_color(theme.warning)
                    .child("No router state found"),
            );
        }

        container
    }

    fn render_timeline_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.ensure_timeline_search(window, cx);
        let theme = cx.theme();
        let filtered = self.filtered_events(cx);
        let item_count = filtered.len();
        let row_height = px(32.0);
        let row_width = px(2000.0);
        let item_sizes = Rc::new(vec![Size::new(row_width, row_height); item_count]);

        let search_entity = self.timeline_search.clone().unwrap();
        let scroll_handle = self.timeline_scroll_handle.clone();

        let bg_color = theme.background;
        let secondary_color = theme.secondary;
        let muted_fg = theme.muted_foreground;
        let fg_color = theme.foreground;
        let info_color = theme.info;
        let border_color = theme.border;

        let filtered_for_list = filtered.clone();
        let entity = cx.entity().clone();

        // Prepare the text to copy when the copy all button is clicked
        let copy_all_text = filtered
            .iter()
            .map(|e| format!("[{}] {:?}", e.timestamp.format("%H:%M:%S%.3f"), e.event))
            .collect::<Vec<_>>()
            .join("\n");

        div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_color(info_color)
                            .font_weight(FontWeight::MEDIUM)
                            .child(Icon::new(IconName::Calendar))
                            .child("Event Timeline"),
                    )
                    .child(Clipboard::new("copy-all-timeline").value(copy_all_text)),
            )
            .child(
                Input::new(&search_entity)
                    .prefix(Icon::new(IconName::Search))
                    .cleanable(true)
                    .small(),
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .border_color(border_color)
                    .rounded(px(6.0))
                    .overflow_hidden()
                    .child(
                        // Horizontal scroll container
                        div().overflow_x_scrollbar().size_full().child(
                            v_virtual_list(
                                entity,
                                "timeline-list",
                                item_sizes,
                                move |_view, visible_range, _window, _cx| {
                                    let events = filtered_for_list.clone();
                                    visible_range
                                        .map(move |ix| {
                                            let event = &events[ix];
                                            let even = ix % 2 == 0;
                                            div()
                                                .w(row_width)
                                                .h(row_height)
                                                .px_3()
                                                .flex()
                                                .items_center()
                                                .gap_3()
                                                .bg(if even {
                                                    bg_color
                                                } else {
                                                    secondary_color.opacity(0.3)
                                                })
                                                .hover(|style| {
                                                    style.bg(secondary_color.opacity(0.6))
                                                })
                                                .child(
                                                    div()
                                                        .min_w(px(100.0))
                                                        .text_color(muted_fg)
                                                        .font_family("monospace")
                                                        .text_sm()
                                                        .child(
                                                            event
                                                                .timestamp
                                                                .format("%H:%M:%S%.3f")
                                                                .to_string(),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .text_color(fg_color)
                                                        .text_sm()
                                                        .child(format!("{:?}", event.event)),
                                                )
                                        })
                                        .collect()
                                },
                            )
                            .track_scroll(&scroll_handle),
                        ),
                    ),
            )
            .when(filtered.is_empty(), |d| {
                d.child(
                    div()
                        .p_3()
                        .text_color(muted_fg)
                        .child("No events match the search"),
                )
            })
    }

    fn render_cache_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .gap_3()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_color(theme.info)
                    .font_weight(FontWeight::MEDIUM)
                    .child(Icon::new(IconName::Inbox))
                    .child("Cache Inspection"),
            )
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("Cache inspection (rs-query integration)"),
            )
    }

    fn render_state_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let state = RouterState::try_global(cx);
        let mut container = div().gap_3().flex().flex_col();

        if let Some(state) = state {
            container = container
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(theme.info)
                        .font_weight(FontWeight::MEDIUM)
                        .child(Icon::new(IconName::Info))
                        .child("Router State"),
                )
                .child(
                    div()
                        .p_3()
                        .bg(theme.secondary)
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .text_color(theme.foreground)
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
            container = container.child(div().text_color(theme.warning).child("No router state"));
        }

        container
    }
}

impl Render for DevtoolsState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.expanded {
            return Button::new("devtools-toggle")
                .icon(IconName::Info)
                .rounded_full()
                .size(px(40.0))
                .shadow_md()
                .absolute()
                .bottom_3()
                .right_3()
                .on_click(cx.listener(|this, _, _, cx| this.toggle_expanded(cx)))
                .into_any_element();
        }

        let theme = cx.theme();
        let viewport = window.viewport_size();
        let panel_width = px(550.0).min(viewport.width - px(20.0));
        let panel_height = px(450.0).min(viewport.height - px(20.0));

        div()
            .absolute()
            .bottom_0()
            .right_0()
            .w(panel_width)
            .h(panel_height)
            .bg(theme.background)
            .text_color(theme.foreground)
            .border_1()
            .border_color(theme.border)
            .rounded_tl(px(8.0))
            .shadow_lg()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .bg(theme.secondary)
                    .rounded_tl(px(8.0))
                    .border_b_1()
                    .border_color(theme.border)
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
                            .icon(IconName::Close)
                            .ghost()
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_expanded(cx))),
                    ),
            )
            .child(
                TabBar::new("devtools-tabs")
                    .selected_index(self.tab_index())
                    .on_click(cx.listener(|this, index, _, cx| {
                        this.set_selected_tab(this.tab_from_index(*index), cx);
                    }))
                    .child(
                        Tab::new()
                            .label("Routes")
                            .prefix(Icon::new(IconName::Folder).ml_1()),
                    )
                    .child(
                        Tab::new()
                            .label("Cache")
                            .prefix(Icon::new(IconName::Inbox).ml_1()),
                    )
                    .child(
                        Tab::new()
                            .label("Timeline")
                            .prefix(Icon::new(IconName::Calendar).ml_1()),
                    )
                    .child(
                        Tab::new()
                            .label("State")
                            .prefix(Icon::new(IconName::Settings).ml_1()),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .p_3()
                    .overflow_y_scrollbar()
                    .child(match self.selected_tab {
                        DevtoolsTab::Routes => self.render_routes_tab(cx).into_any_element(),
                        DevtoolsTab::Cache => self.render_cache_tab(cx).into_any_element(),
                        DevtoolsTab::Timeline => {
                            self.render_timeline_tab(window, cx).into_any_element()
                        }
                        DevtoolsTab::State => self.render_state_tab(cx).into_any_element(),
                    }),
            )
            .into_any_element()
    }
}

#[derive(Clone)]
pub struct NaviDevtools {
    state: Entity<DevtoolsState>,
}

impl NaviDevtools {
    pub fn new(cx: &mut App) -> Self {
        Self {
            state: cx.new(DevtoolsState::new),
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
