use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, FontWeight, KeyBinding, Render, RenderOnce,
    Size, StyledText, Subscription, TextStyle, Window, actions, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, VirtualListScrollHandle,
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    tab::{Tab, TabBar},
    v_virtual_list,
};
use navi_router::{
    RouterEvent, RouterState,
    event_bus::{self, TimedEvent},
};
use std::rc::Rc;

actions!(devtools, [ToggleDevtools]);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RouterEventType {
    All,
    BeforeNavigate,
    BeforeLoad,
    Load,
    BeforeRouteMount,
    Resolved,
    Rendered,
}

impl RouterEventType {
    fn from_event(event: &RouterEvent) -> Self {
        match event {
            RouterEvent::BeforeNavigate { .. } => Self::BeforeNavigate,
            RouterEvent::BeforeLoad { .. } => Self::BeforeLoad,
            RouterEvent::Load { .. } => Self::Load,
            RouterEvent::BeforeRouteMount { .. } => Self::BeforeRouteMount,
            RouterEvent::Resolved { .. } => Self::Resolved,
            RouterEvent::Rendered { .. } => Self::Rendered,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::All => "All Events",
            Self::BeforeNavigate => "BeforeNavigate",
            Self::BeforeLoad => "BeforeLoad",
            Self::Load => "Load",
            Self::BeforeRouteMount => "BeforeRouteMount",
            Self::Resolved => "Resolved",
            Self::Rendered => "Rendered",
        }
    }
}

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
    event_log: Vec<TimedEvent>,
    timeline_search: Option<Entity<InputState>>,
    timeline_scroll_handle: VirtualListScrollHandle,
    _subscription: Subscription,
    last_log_len: usize,
    highlight_new_count: usize,
    filter_event_type: RouterEventType,
    focus_handle: FocusHandle,
}

impl EventEmitter<()> for DevtoolsState {}

impl DevtoolsState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Bind keyboard shortcuts globally
        cx.bind_keys([KeyBinding::new(
            "cmd-shift-d",
            ToggleDevtools,
            Some("Devtools"),
        )]);
        cx.bind_keys([KeyBinding::new(
            "ctrl-shift-d",
            ToggleDevtools,
            Some("Devtools"),
        )]);

        let subscription = cx.observe_global::<RouterState>(move |this, cx| {
            this.refresh_log(cx);
        });

        let mut this = Self {
            expanded: true,
            selected_tab: DevtoolsTab::Timeline,
            event_log: Vec::new(),
            timeline_search: None,
            timeline_scroll_handle: VirtualListScrollHandle::new(),
            _subscription: subscription,
            last_log_len: 0,
            highlight_new_count: 0,
            filter_event_type: RouterEventType::All,
            focus_handle: cx.focus_handle(),
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
            self.timeline_search = Some(state);
        }
    }

    fn refresh_log(&mut self, cx: &mut Context<Self>) {
        let new_log = event_bus::get_event_log(cx);
        let new_len = new_log.len();
        let new_events_count = new_len.saturating_sub(self.last_log_len);
        self.highlight_new_count = new_events_count;
        self.last_log_len = new_len;
        self.event_log = new_log;
        cx.notify();
    }

    fn filtered_events(&self, cx: &App) -> Vec<TimedEvent> {
        let query = self
            .timeline_search
            .as_ref()
            .map(|s| s.read(cx).value().to_lowercase())
            .unwrap_or_default();
        self.event_log
            .iter()
            .filter(|e| {
                let event_type = RouterEventType::from_event(&e.event);
                (self.filter_event_type == RouterEventType::All
                    || event_type == self.filter_event_type)
                    && (query.is_empty()
                        || format!("{:?}", e.event).to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }

    fn set_selected_tab(&mut self, tab: DevtoolsTab, cx: &mut Context<Self>) {
        self.selected_tab = tab;
        cx.notify();
    }

    fn toggle_expanded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        // When collapsing, force focus onto the small button
        // so the "Devtools" key_context stays active for the shortcut.
        if !self.expanded {
            self.focus_handle.focus(window, cx);
        }
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
            _ => DevtoolsTab::Timeline,
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
        let mut filtered = self.filtered_events(cx);
        let mut deltas = Vec::new();
        if !filtered.is_empty() {
            deltas.push(0.0);
            for i in 1..filtered.len() {
                let delta = filtered[i].timestamp - filtered[i - 1].timestamp;
                deltas.push(delta.num_milliseconds() as f64);
            }
        }
        filtered.reverse();
        deltas.reverse();

        let item_count = filtered.len();
        let row_height = px(32.0);
        let row_width = px(2500.0);
        let item_sizes = Rc::new(vec![Size::new(row_width, row_height); item_count]);

        let search_entity = self.timeline_search.clone().unwrap();
        let scroll_handle = self.timeline_scroll_handle.clone();

        let bg_color = theme.background;
        let secondary_color = theme.secondary;
        let muted_fg = theme.muted_foreground;
        let fg_color = theme.foreground;
        let info_color = theme.info;
        let border_color = theme.border;
        let highlight_color = theme.success.opacity(0.25);
        let search_highlight_bg = theme.warning;
        let warning_color = theme.warning;

        let filtered_for_list = filtered.clone();
        let deltas_for_list = deltas.clone();
        let entity = cx.entity().clone();
        let highlight_count = self.highlight_new_count;

        let search_query = self
            .timeline_search
            .as_ref()
            .map(|s| s.read(cx).value())
            .unwrap_or_default();

        let copy_all_text = filtered
            .iter()
            .map(|e| format!("[{}] {:?}", e.timestamp.format("%H:%M:%S%.3f"), e.event))
            .collect::<Vec<_>>()
            .join("\n");

        let weak_self = cx.entity().downgrade();

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
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("filter-button")
                                    .label(format!("Filter: {}", self.filter_event_type.label()))
                                    .ghost()
                                    .small()
                                    .dropdown_menu({
                                        let weak_self = weak_self.clone();
                                        move |menu, _window, cx| {
                                            let mut menu = menu;
                                            for event_type in [
                                                RouterEventType::All,
                                                RouterEventType::BeforeNavigate,
                                                RouterEventType::BeforeLoad,
                                                RouterEventType::Load,
                                                RouterEventType::BeforeRouteMount,
                                                RouterEventType::Resolved,
                                                RouterEventType::Rendered,
                                            ] {
                                                let is_checked = weak_self
                                                    .upgrade()
                                                    .map(|this| {
                                                        this.read(cx).filter_event_type
                                                            == event_type
                                                    })
                                                    .unwrap_or(false);
                                                menu = menu.item(
                                                    PopupMenuItem::new(event_type.label())
                                                        .checked(is_checked)
                                                        .on_click({
                                                            let weak_self = weak_self.clone();
                                                            let event_type = event_type;
                                                            move |_, _window, cx| {
                                                                weak_self
                                                                    .update(cx, |this, inner_cx| {
                                                                        this.filter_event_type =
                                                                            event_type;
                                                                        inner_cx.notify();
                                                                    })
                                                                    .ok();
                                                            }
                                                        }),
                                                );
                                            }
                                            menu
                                        }
                                    }),
                            )
                            .child(
                                Clipboard::new("copy-all-timeline")
                                    .value(copy_all_text)
                                    .tooltip("Copy all events"),
                            )
                            .child(
                                Button::new("clear-timeline")
                                    .icon(IconName::Delete)
                                    .ghost()
                                    .xsmall()
                                    .tooltip("Clear all events")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        event_bus::clear_event_log(cx);
                                        this.refresh_log(cx);
                                    })),
                            ),
                    ),
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
                    .child(if filtered.is_empty() {
                        let empty_msg = if self.event_log.is_empty() {
                            "No events recorded yet"
                        } else {
                            "No events match the search or filter"
                        };
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().text_color(muted_fg).child(empty_msg))
                            .into_any_element()
                    } else {
                        div()
                            .overflow_x_scrollbar()
                            .size_full()
                            .child(
                                v_virtual_list(
                                    entity,
                                    "timeline-list",
                                    item_sizes,
                                    move |_view, visible_range, _window, _cx| {
                                        let events = filtered_for_list.clone();
                                        let deltas = deltas_for_list.clone();
                                        let search_query = search_query.clone();
                                        visible_range
                                            .map(move |ix| {
                                                let event = &events[ix];
                                                let delta = deltas[ix];
                                                let even = ix % 2 == 0;
                                                let is_new = ix < highlight_count;
                                                let event_text = format!("{:?}", event.event);
                                                let text_len = event_text.len();

                                                let styled_text = if search_query.is_empty() {
                                                    StyledText::new(event_text).with_runs(vec![
                                                        TextStyle {
                                                            color: fg_color,
                                                            ..Default::default()
                                                        }
                                                        .to_run(text_len),
                                                    ])
                                                } else {
                                                    let query_lower = search_query.to_lowercase();
                                                    let text_lower = event_text.to_lowercase();
                                                    let mut runs = Vec::new();
                                                    let mut last_end = 0;
                                                    let mut start = 0;

                                                    while let Some(pos) =
                                                        text_lower[start..].find(&query_lower)
                                                    {
                                                        let abs_pos = start + pos;
                                                        if abs_pos > last_end {
                                                            runs.push(
                                                                TextStyle {
                                                                    color: fg_color,
                                                                    ..Default::default()
                                                                }
                                                                .to_run(abs_pos - last_end),
                                                            );
                                                        }
                                                        runs.push(
                                                            TextStyle {
                                                                color: fg_color,
                                                                background_color: Some(
                                                                    search_highlight_bg,
                                                                ),
                                                                ..Default::default()
                                                            }
                                                            .to_run(query_lower.len()),
                                                        );
                                                        last_end = abs_pos + query_lower.len();
                                                        start = abs_pos + query_lower.len();
                                                    }
                                                    if last_end < text_len {
                                                        runs.push(
                                                            TextStyle {
                                                                color: fg_color,
                                                                ..Default::default()
                                                            }
                                                            .to_run(text_len - last_end),
                                                        );
                                                    }
                                                    StyledText::new(event_text).with_runs(runs)
                                                };

                                                div()
                                                    .w(row_width)
                                                    .h(row_height)
                                                    .px_3()
                                                    .flex()
                                                    .items_center()
                                                    .gap_3()
                                                    .bg(if is_new {
                                                        highlight_color
                                                    } else if even {
                                                        bg_color
                                                    } else {
                                                        secondary_color.opacity(0.3)
                                                    })
                                                    .hover(|style| {
                                                        style.bg(secondary_color.opacity(0.6))
                                                    })
                                                    .child(
                                                        div()
                                                            .min_w(px(80.0))
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
                                                            .min_w(px(60.0))
                                                            .text_right()
                                                            .text_color(if delta > 100.0 {
                                                                warning_color
                                                            } else {
                                                                muted_fg
                                                            })
                                                            .text_sm()
                                                            .child(if ix == 0 {
                                                                div().child("—")
                                                            } else {
                                                                div().child(format!(
                                                                    "+{:.0}ms",
                                                                    delta
                                                                ))
                                                            }),
                                                    )
                                                    .child(div().flex_1().child(styled_text))
                                            })
                                            .collect()
                                    },
                                )
                                .track_scroll(&scroll_handle),
                            )
                            .into_any_element()
                    }),
            )
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
            return div()
                .absolute()
                .bottom_3()
                .right_3()
                .track_focus(&self.focus_handle)
                .key_context("Devtools")
                .on_action(cx.listener(|this, _: &ToggleDevtools, window, cx| {
                    this.toggle_expanded(window, cx);
                }))
                .child(
                    Button::new("devtools-toggle")
                        .icon(IconName::Info)
                        .ghost()
                        .rounded_full()
                        .size(px(40.0))
                        .shadow_md()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_expanded(window, cx);
                        })),
                )
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
            .track_focus(&self.focus_handle)
            .key_context("Devtools")
            .on_action(cx.listener(|this, _: &ToggleDevtools, window, cx| {
                this.toggle_expanded(window, cx);
            }))
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
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_expanded(window, cx);
                            })),
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
