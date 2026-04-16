use gpui::{
    App, Context, Div, Entity, EventEmitter, FocusHandle, FontWeight, Hsla, KeyBinding,
    MouseButton, Render, Size, StyledText, Subscription, TextStyle, Window, actions, div,
    prelude::*, px,
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
    Navigator, RouterEvent, RouterState,
    event_bus::{self, TimedEvent},
};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

actions!(
    devtools,
    [
        ToggleDevtools,
        SwitchToTab1,
        SwitchToTab2,
        SwitchToTab3,
        SwitchToTab4,
        FocusTimelineSearch
    ]
);

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

// ---------------------------------------------------------------------------
// Event display types
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct EventDetail {
    from_pathname: Option<String>,
    from_search: Option<String>,
    to_pathname: String,
    to_search: Option<String>,
}

fn format_search(search: &serde_json::Value) -> String {
    if let serde_json::Value::Object(map) = search {
        if map.is_empty() {
            return String::new();
        }
        let pairs: Vec<String> = map
            .iter()
            .filter_map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                Some(format!("{}={}", k, val))
            })
            .collect();
        if pairs.is_empty() {
            String::new()
        } else {
            format!("?{}", pairs.join("&"))
        }
    } else {
        String::new()
    }
}

fn extract_event_detail(event: &RouterEvent) -> EventDetail {
    match event {
        RouterEvent::BeforeNavigate { from, to }
        | RouterEvent::BeforeLoad { from, to }
        | RouterEvent::Load { from, to }
        | RouterEvent::BeforeRouteMount { from, to }
        | RouterEvent::Resolved { from, to }
        | RouterEvent::Rendered { from, to } => {
            let from_search_str = from
                .as_ref()
                .map(|l| format_search(&l.search))
                .unwrap_or_default();
            let to_search_str = format_search(&to.search);

            EventDetail {
                from_pathname: from.as_ref().map(|l| l.pathname.clone()),
                from_search: if from_search_str.is_empty() {
                    None
                } else {
                    Some(from_search_str)
                },
                to_pathname: to.pathname.clone(),
                to_search: if to_search_str.is_empty() {
                    None
                } else {
                    Some(to_search_str)
                },
            }
        }
    }
}

#[derive(Clone)]
struct EventDisplay {
    timestamp_str: String,
    badge: &'static str,
    badge_color: Hsla,
    text: String,
    detail: EventDetail,
}

fn format_event_text(event: &RouterEvent) -> String {
    match event {
        RouterEvent::BeforeNavigate { from, to } => {
            let from_path = from
                .as_ref()
                .map(|l| format!("{}{}", l.pathname, format_search(&l.search)))
                .unwrap_or_else(|| "?".to_string());
            let to_path = format!("{}{}", to.pathname, format_search(&to.search));
            format!("{} → {}", from_path, to_path)
        }
        RouterEvent::BeforeLoad { to, .. }
        | RouterEvent::Load { to, .. }
        | RouterEvent::BeforeRouteMount { to, .. }
        | RouterEvent::Resolved { to, .. }
        | RouterEvent::Rendered { to, .. } => {
            format!("{}{}", to.pathname, format_search(&to.search))
        }
    }
}

fn build_event_display(
    event: &RouterEvent,
    timestamp_str: String,
    colors: &EventColors,
) -> EventDisplay {
    let detail = extract_event_detail(event);

    match event {
        RouterEvent::BeforeNavigate { from, to } => {
            let from_path = from
                .as_ref()
                .map(|l| format!("{}{}", l.pathname, format_search(&l.search)))
                .unwrap_or_else(|| "?".to_string());
            let to_path = format!("{}{}", to.pathname, format_search(&to.search));
            EventDisplay {
                timestamp_str,
                badge: "NAV",
                badge_color: colors.primary,
                text: format!("{} → {}", from_path, to_path),
                detail,
            }
        }
        RouterEvent::BeforeLoad { to, .. } => EventDisplay {
            timestamp_str,
            badge: "BLD",
            badge_color: colors.warning,
            text: format!("{}{}", to.pathname, format_search(&to.search)),
            detail,
        },
        RouterEvent::Load { to, .. } => EventDisplay {
            timestamp_str,
            badge: "LOAD",
            badge_color: colors.warning,
            text: format!("{}{}", to.pathname, format_search(&to.search)),
            detail,
        },
        RouterEvent::BeforeRouteMount { to, .. } => EventDisplay {
            timestamp_str,
            badge: "MNT",
            badge_color: colors.info,
            text: format!("{}{}", to.pathname, format_search(&to.search)),
            detail,
        },
        RouterEvent::Resolved { to, .. } => EventDisplay {
            timestamp_str,
            badge: "OK",
            badge_color: colors.success,
            text: format!("{}{}", to.pathname, format_search(&to.search)),
            detail,
        },
        RouterEvent::Rendered { to, .. } => EventDisplay {
            timestamp_str,
            badge: "REN",
            badge_color: colors.info,
            text: format!("{}{}", to.pathname, format_search(&to.search)),
            detail,
        },
    }
}

struct EventColors {
    primary: Hsla,
    success: Hsla,
    warning: Hsla,
    info: Hsla,
}

// ---------------------------------------------------------------------------
// DevtoolsState
// ---------------------------------------------------------------------------

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
    nav_input: Option<Entity<InputState>>,
    selected_event_detail: Option<EventDetail>,
}

impl EventEmitter<()> for DevtoolsState {}

impl DevtoolsState {
    pub fn new(cx: &mut Context<Self>) -> Self {
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
        cx.bind_keys([KeyBinding::new(
            "cmd-shift-tab",
            ToggleDevtools,
            Some("Devtools"),
        )]);
        cx.bind_keys([KeyBinding::new(
            "ctrl-shift-tab",
            ToggleDevtools,
            Some("Devtools"),
        )]);
        cx.bind_keys([KeyBinding::new("cmd-1", SwitchToTab1, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new("ctrl-1", SwitchToTab1, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new("cmd-2", SwitchToTab2, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new("ctrl-2", SwitchToTab2, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new("cmd-3", SwitchToTab3, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new("ctrl-3", SwitchToTab3, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new("cmd-4", SwitchToTab4, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new("ctrl-4", SwitchToTab4, Some("Devtools"))]);
        cx.bind_keys([KeyBinding::new(
            "cmd-f",
            FocusTimelineSearch,
            Some("Devtools"),
        )]);
        cx.bind_keys([KeyBinding::new(
            "ctrl-f",
            FocusTimelineSearch,
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
            nav_input: None,
            selected_event_detail: None,
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

    fn ensure_nav_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.nav_input.is_none() {
            let state = cx.new(|cx| {
                let mut s = InputState::new(window, cx);
                s.set_placeholder("Navigate to path (e.g. /users/42)...", window, cx);
                s
            });
            self.nav_input = Some(state);
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
                        || format_event_text(&e.event).to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }

    fn set_selected_tab(&mut self, tab: DevtoolsTab, cx: &mut Context<Self>) {
        self.selected_tab = tab;
        self.selected_event_detail = None;
        cx.notify();
    }

    fn toggle_expanded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
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

    fn select_event(&mut self, detail: EventDetail, cx: &mut Context<Self>) {
        if self.selected_event_detail.as_ref() == Some(&detail) {
            self.selected_event_detail = None;
        } else {
            self.selected_event_detail = Some(detail);
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // Routes tab
    // -----------------------------------------------------------------------

    fn render_routes_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.ensure_nav_input(window, cx);

        let theme = cx.theme();
        let state = RouterState::try_global(cx);
        let mut container = div().gap_3().flex().flex_col();

        if let Some(state) = state {
            let loc = state.current_location();
            let search_str = format_search(&loc.search);
            let full_path = if search_str.is_empty() {
                loc.pathname.clone()
            } else {
                format!("{}{}", loc.pathname, search_str)
            };

            // Current Location with Copy button
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
                                    .justify_between()
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
                                    .child(
                                        Clipboard::new("copy-current-path")
                                            .value(full_path)
                                            .tooltip("Copy path"),
                                    ),
                            )
                            .child(format!("Path: {}", loc.pathname))
                            .child(format!(
                                "Search: {}",
                                if search_str.is_empty() {
                                    "None".to_string()
                                } else {
                                    search_str
                                }
                            )),
                    ),
            );

            // Direct Navigation Input
            let nav_input_entity = self.nav_input.clone().unwrap();
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
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .text_color(theme.info)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(Icon::new(IconName::ArrowRight))
                                    .child("Navigate to Path"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Input::new(&nav_input_entity)
                                            .prefix(Icon::new(IconName::Search))
                                            .cleanable(true)
                                            .small(),
                                    )
                                    .child(
                                        Button::new("nav-go-btn").label("Go").compact().on_click(
                                            cx.listener(|this, _, window, cx| {
                                                let val = this
                                                    .nav_input
                                                    .as_ref()
                                                    .map(|i| i.read(cx).value().to_string())
                                                    .unwrap_or_default();
                                                let trimmed = val.trim();
                                                if !trimmed.is_empty() && trimmed.starts_with('/') {
                                                    let nav = Navigator::new(
                                                        window.window_handle().into(),
                                                    );
                                                    nav.push(trimmed, cx);
                                                }
                                            }),
                                        ),
                                    ),
                            ),
                    ),
            );

            // --- Matched Route Chain ---
            let matched_info = state
                .current_match
                .as_ref()
                .map(|(params, node)| (node.id.clone(), format!("{:?}", params)));

            let node_infos: Vec<(String, String, bool, bool, bool, Option<String>)> = state
                .route_tree
                .all_nodes()
                .map(|n| {
                    (
                        n.id.clone(),
                        n.pattern.raw.clone(),
                        n.is_layout,
                        n.is_index,
                        n.has_loader,
                        n.parent.clone(),
                    )
                })
                .collect();

            let parent_of: HashMap<String, String> = node_infos
                .iter()
                .filter_map(|(id, _, _, _, _, p)| Some((id.clone(), p.clone()?)))
                .collect();

            fn node_depth(id: &str, parent_of: &HashMap<String, String>) -> usize {
                let mut d = 0;
                let mut cur = id;
                while let Some(p) = parent_of.get(cur) {
                    d += 1;
                    cur = p;
                }
                d
            }

            let matched_leaf_id: Option<String> = matched_info.as_ref().map(|(id, _)| id.clone());
            let matched_chain: HashSet<String> = if let Some(ref leaf_id) = matched_leaf_id {
                let mut chain = HashSet::new();
                let mut cur: &str = leaf_id.as_str();
                loop {
                    chain.insert(cur.to_string());
                    cur = match parent_of.get(cur) {
                        Some(p) => p,
                        None => break,
                    };
                }
                chain
            } else {
                HashSet::new()
            };

            struct ChainRow {
                id: String,
                pattern: String,
                tag: &'static str,
                is_leaf: bool,
                depth_f32: f32,
            }

            let chain_rows: Vec<ChainRow> = if let Some((ref leaf_id, _)) = matched_info {
                let mut chain_ids: Vec<String> = Vec::new();
                let mut cur: &str = leaf_id.as_str();
                loop {
                    chain_ids.push(cur.to_string());
                    cur = match parent_of.get(cur) {
                        Some(p) => p,
                        None => break,
                    };
                }
                chain_ids.reverse();

                chain_ids
                    .iter()
                    .enumerate()
                    .map(|(i, chain_id)| {
                        let info = node_infos.iter().find(|(id, _, _, _, _, _)| id == chain_id);
                        let (pattern, is_layout, is_index) = info
                            .map(|(_, p, l, idx, _, _)| (p.clone(), *l, *idx))
                            .unwrap_or(("?".to_string(), false, false));

                        let tag = if is_layout {
                            "layout"
                        } else if is_index {
                            "index"
                        } else {
                            "leaf"
                        };

                        ChainRow {
                            id: chain_id.clone(),
                            pattern,
                            tag,
                            is_leaf: i == chain_ids.len() - 1,
                            depth_f32: 16.0 * i as f32,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let params_display = matched_info
                .as_ref()
                .map(|(_, p)| format!("Params: {}", p))
                .unwrap_or_else(|| "No route matched!".to_string());
            let has_match = matched_info.is_some();

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
                                .child("Matched Route Chain"),
                        );

                        for row in chain_rows {
                            let arrow = if row.is_leaf { "→" } else { "└" };
                            let arrow_color = if row.is_leaf {
                                theme.primary
                            } else {
                                theme.muted_foreground
                            };

                            let row_div = div()
                                .pl(px(row.depth_f32))
                                .pr_2()
                                .py(px(2.0))
                                .rounded(px(3.0))
                                .when(row.is_leaf, |d| d.bg(theme.primary.opacity(0.15)))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w(px(16.0))
                                                .text_color(arrow_color)
                                                .text_size(px(11.0))
                                                .child(arrow),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.foreground)
                                                .text_size(px(12.0))
                                                .child(row.id),
                                        )
                                        .child(
                                            div()
                                                .px_1()
                                                .rounded(px(3.0))
                                                .bg(theme.muted_foreground.opacity(0.15))
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(10.0))
                                                .child(row.tag),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child(row.pattern),
                                        ),
                                );

                            card = card.child(row_div);
                        }

                        if has_match {
                            card = card.child(
                                div()
                                    .mt_1()
                                    .pl(px(16.0))
                                    .text_color(theme.muted_foreground)
                                    .text_size(px(11.0))
                                    .child(params_display),
                            );
                        } else {
                            card = card
                                .child(div().text_color(theme.warning).child("No route matched!"));
                        }

                        card
                    }),
            );

            // --- Full Route Tree ---
            let mut sorted_nodes: Vec<(String, String, bool, bool, bool, usize)> = node_infos
                .iter()
                .map(|(id, pattern, is_layout, is_index, has_loader, _)| {
                    let depth = node_depth(id, &parent_of);
                    (
                        id.clone(),
                        pattern.clone(),
                        *is_layout,
                        *is_index,
                        *has_loader,
                        depth,
                    )
                })
                .collect();
            sorted_nodes.sort_by_key(|(id, _, _, _, _, depth)| (*depth, id.clone()));

            let total_routes = sorted_nodes.len();
            let layout_count = sorted_nodes.iter().filter(|(_, _, l, _, _, _)| *l).count();
            let loader_count = sorted_nodes
                .iter()
                .filter(|(_, _, _, _, ld, _)| *ld)
                .count();

            container = container
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
                                .text_color(theme.info)
                                .font_weight(FontWeight::MEDIUM)
                                .child(Icon::new(IconName::FolderOpen))
                                .child("Route Tree"),
                        )
                        .child(
                            div()
                                .text_color(theme.muted_foreground)
                                .text_size(px(11.0))
                                .child(format!(
                                    "{} routes · {} layouts · {} with loaders",
                                    total_routes, layout_count, loader_count
                                )),
                        ),
                )
                .children(sorted_nodes.iter().map(
                    |(id, pattern, is_layout, is_index, has_loader, depth)| {
                        let is_in_chain = matched_chain.contains(id);
                        let is_leaf_match = matched_leaf_id.as_deref() == Some(id.as_str());
                        let indent_px = px(16.0 * *depth as f32);

                        let mut tags: Vec<&'static str> = Vec::new();
                        if *is_layout {
                            tags.push("layout");
                        }
                        if *is_index {
                            tags.push("index");
                        }
                        if *has_loader {
                            tags.push("loader");
                        }

                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .pl(indent_px)
                            .pr_2()
                            .py(px(3.0))
                            .rounded(px(4.0))
                            .when(is_in_chain, |d| {
                                d.bg(if is_leaf_match {
                                    theme.primary.opacity(0.2)
                                } else {
                                    theme.success.opacity(0.1)
                                })
                            })
                            .hover(|style| style.bg(theme.secondary.opacity(0.5)))
                            .child(
                                div()
                                    .min_w(px(110.0))
                                    .text_color(if is_leaf_match {
                                        theme.primary
                                    } else if is_in_chain {
                                        theme.success
                                    } else {
                                        theme.foreground
                                    })
                                    .text_size(px(11.0))
                                    .font_weight(if is_leaf_match {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .child(id.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(theme.muted_foreground)
                                    .text_size(px(11.0))
                                    .child(pattern.clone()),
                            )
                            .when(!tags.is_empty(), |d| {
                                d.child(div().flex().gap_1().children(tags.into_iter().map(
                                    |tag| {
                                        div()
                                            .px_1()
                                            .rounded(px(2.0))
                                            .bg(theme.muted_foreground.opacity(0.1))
                                            .text_color(theme.muted_foreground)
                                            .text_size(px(9.0))
                                            .child(tag)
                                    },
                                )))
                            })
                    },
                ));
        } else {
            container = container.child(
                div()
                    .text_color(theme.warning)
                    .child("No router state found"),
            );
        }

        container
    }

    // -----------------------------------------------------------------------
    // Timeline tab
    // -----------------------------------------------------------------------

    fn render_timeline_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.ensure_timeline_search(window, cx);

        let theme = cx.theme();
        let colors = EventColors {
            primary: theme.primary,
            success: theme.success,
            warning: theme.warning,
            info: theme.info,
        };

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

        let event_displays: Vec<EventDisplay> = filtered
            .iter()
            .map(|e| {
                build_event_display(
                    &e.event,
                    e.timestamp.format("%H:%M:%S%.3f").to_string(),
                    &colors,
                )
            })
            .collect();

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
        let primary_bg = theme.primary.opacity(0.15);

        let displays_for_list = event_displays;
        let deltas_for_list = deltas;
        let entity = cx.entity().clone();
        let highlight_count = self.highlight_new_count;
        let selected_detail = self.selected_event_detail.clone();

        let search_query = self
            .timeline_search
            .as_ref()
            .map(|s| s.read(cx).value())
            .unwrap_or_default();

        let copy_all_text = displays_for_list
            .iter()
            .map(|d| format!("[{}] {}", d.badge, d.text))
            .collect::<Vec<_>>()
            .join("\n");

        let weak_self = cx.entity().downgrade();

        let has_detail = selected_detail.is_some();

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
                    .min_h_0()
                    .border_1()
                    .border_color(border_color)
                    .rounded(px(6.0))
                    .overflow_hidden()
                    .child(if displays_for_list.is_empty() {
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
                            .child({
                                let entity_for_list = entity.clone();
                                v_virtual_list(
                                    entity_for_list,
                                    "timeline-list",
                                    item_sizes,
                                    move |_view, visible_range, _window, _cx| {
                                        let displays = displays_for_list.clone();
                                        let deltas = deltas_for_list.clone();
                                        let search_query = search_query.clone();
                                        let displays_for_click = displays_for_list.clone();
                                        let selected_detail_inner = selected_detail.clone();
                                        let entity_inner = entity.clone();
                                        visible_range
                                            .map(move |ix| {
                                                let display = &displays[ix];
                                                let delta = deltas[ix];
                                                let even = ix % 2 == 0;
                                                let is_new = ix < highlight_count;
                                                let is_selected = selected_detail_inner
                                                    .as_ref()
                                                    .map(|d| d == &display.detail)
                                                    .unwrap_or(false);
                                                let event_text = &display.text;
                                                let text_len = event_text.len();

                                                let styled_text = if search_query.is_empty() {
                                                    StyledText::new(event_text.clone()).with_runs(
                                                        vec![
                                                            TextStyle {
                                                                color: fg_color,
                                                                ..Default::default()
                                                            }
                                                            .to_run(text_len),
                                                        ],
                                                    )
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
                                                    StyledText::new(event_text.clone())
                                                        .with_runs(runs)
                                                };

                                                let delta_text = if ix == 0 {
                                                    "—".to_string()
                                                } else {
                                                    format!("+{:.0}ms", delta)
                                                };

                                                div()
                                                    .w(row_width)
                                                    .h(row_height)
                                                    .px_3()
                                                    .flex()
                                                    .items_center()
                                                    .gap_3()
                                                    .cursor_pointer()
                                                    .bg(if is_selected {
                                                        primary_bg
                                                    } else if is_new {
                                                        highlight_color
                                                    } else if even {
                                                        bg_color
                                                    } else {
                                                        secondary_color.opacity(0.3)
                                                    })
                                                    .hover(|style| {
                                                        style.bg(secondary_color.opacity(0.6))
                                                    })
                                                    .on_mouse_down(MouseButton::Left, {
                                                        let entity = entity_inner.clone();
                                                        let displays = displays_for_click.clone();
                                                        move |_event, _window, cx| {
                                                            if let Some(display) = displays.get(ix)
                                                            {
                                                                entity.update(cx, |state, cx| {
                                                                    state.select_event(
                                                                        display.detail.clone(),
                                                                        cx,
                                                                    );
                                                                });
                                                            }
                                                        }
                                                    })
                                                    .child(
                                                        div()
                                                            .min_w(px(80.0))
                                                            .text_color(muted_fg)
                                                            .font_family("monospace")
                                                            .text_size(px(11.0))
                                                            .child(display.timestamp_str.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .min_w(px(36.0))
                                                            .text_color(display.badge_color)
                                                            .font_weight(FontWeight::BOLD)
                                                            .text_size(px(10.0))
                                                            .child(display.badge),
                                                    )
                                                    .child(
                                                        div()
                                                            .min_w(px(56.0))
                                                            .text_right()
                                                            .text_color(if delta > 100.0 {
                                                                warning_color
                                                            } else {
                                                                muted_fg
                                                            })
                                                            .text_size(px(10.0))
                                                            .child(delta_text),
                                                    )
                                                    .child(div().flex_1().child(styled_text))
                                            })
                                            .collect()
                                    },
                                )
                                .track_scroll(&scroll_handle)
                            })
                            .into_any_element()
                    }),
            )
            .when(has_detail, |d| d.child(self.render_event_detail_panel(cx)))
    }

    fn render_event_detail_panel(&self, cx: &Context<Self>) -> Div {
        let theme = cx.theme();
        let detail = self
            .selected_event_detail
            .as_ref()
            .cloned()
            .unwrap_or_else(|| EventDetail {
                from_pathname: None,
                from_search: None,
                to_pathname: "?".to_string(),
                to_search: None,
            });

        let label_row = |label: &str, value: String| -> Div {
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .min_w(px(70.0))
                        .text_color(theme.muted_foreground)
                        .text_size(px(10.0))
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(11.0))
                        .child(value),
                )
        };

        let mut panel = div()
            .mt_1()
            .p_3()
            .bg(theme.secondary)
            .rounded(px(6.0))
            .border_1()
            .border_color(theme.border)
            .border_t_1()
            .border_color(theme.muted_foreground.opacity(0.3))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_color(theme.muted_foreground)
                    .text_size(px(10.0))
                    .child("EVENT PAYLOAD")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .ml_auto()
                            .cursor_pointer()
                            .text_color(theme.muted_foreground)
                            .hover(|s| s.text_color(theme.foreground))
                            .child("✕")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.selected_event_detail = None;
                                    cx.notify();
                                }),
                            ),
                    ),
            );

        if let Some(from) = &detail.from_pathname {
            panel = panel.child(label_row("From:", from.clone()));
            if let Some(search) = &detail.from_search {
                panel = panel.child(label_row("  Search:", search.clone()));
            }
        } else {
            panel = panel.child(label_row("From:", "(initial navigation)".to_string()));
        }

        panel = panel.child(label_row("To:", detail.to_pathname.clone()));
        if let Some(search) = &detail.to_search {
            panel = panel.child(label_row("  Search:", search.clone()));
        }

        panel
    }

    // -----------------------------------------------------------------------
    // Cache tab
    // -----------------------------------------------------------------------

    fn render_cache_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let state = RouterState::try_global(cx);
        let mut container = div().gap_3().flex().flex_col();

        container = container.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .text_color(theme.info)
                .font_weight(FontWeight::MEDIUM)
                .child(Icon::new(IconName::Inbox))
                .child("Cache Inspection"),
        );

        if let Some(state) = state {
            let loader_routes: Vec<(String, String)> = state
                .route_tree
                .all_nodes()
                .filter(|n| n.has_loader)
                .map(|n| (n.id.clone(), n.pattern.raw.clone()))
                .collect();

            if loader_routes.is_empty() {
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
                                .gap_2()
                                .child(
                                    div()
                                        .text_color(theme.muted_foreground)
                                        .child("No routes with loaders registered."),
                                )
                                .child(
                                    div()
                                        .text_color(theme.muted_foreground)
                                        .opacity(0.7)
                                        .text_size(px(11.0))
                                        .child(
                                            "Use define_route! with a loader: to enable cache tracking.",
                                        ),
                                ),
                        ),
                );
            } else {
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
                                .gap_2()
                                .child(
                                    div()
                                        .text_color(theme.muted_foreground)
                                        .text_size(px(11.0))
                                        .child(format!(
                                            "{} route(s) with loaders (cache integration pending):",
                                            loader_routes.len()
                                        )),
                                )
                                .children(loader_routes.into_iter().map(|(id, pattern)| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .pl_2()
                                        .child(
                                            div()
                                                .w(px(8.0))
                                                .h(px(8.0))
                                                .rounded_full()
                                                .bg(theme.warning.opacity(0.5)),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.foreground)
                                                .text_size(px(11.0))
                                                .child(id),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child(pattern),
                                        )
                                })),
                        ),
                );
            }

            container = container.child(
                div()
                    .mt_2()
                    .p_3()
                    .bg(theme.secondary.opacity(0.5))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.border.opacity(0.5))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_color(theme.muted_foreground)
                            .opacity(0.6)
                            .text_size(px(11.0))
                            .child("Planned features:")
                            .child("• Per-route cache status (fresh / stale / loading / error)")
                            .child("• Stale-time & GC-time countdowns")
                            .child("• Manual cache invalidation per route")
                            .child("• Cache size statistics"),
                    ),
            );
        } else {
            container = container.child(
                div()
                    .text_color(theme.warning)
                    .child("No router state found"),
            );
        }

        container
    }

    // -----------------------------------------------------------------------
    // State tab
    // -----------------------------------------------------------------------

    fn render_state_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let state = RouterState::try_global(cx);
        let mut container = div().gap_3().flex().flex_col();

        if let Some(state) = state {
            let loc = state.current_location();
            let total_routes = state.route_tree.all_nodes().count();
            let layout_count = state.route_tree.all_nodes().filter(|n| n.is_layout).count();
            let loader_count = state
                .route_tree
                .all_nodes()
                .filter(|n| n.has_loader)
                .count();
            let can_back = state.history.can_go_back();
            let can_forward = state.history.can_go_forward();
            let blocker_count = state.blockers.len();
            let has_blockers = !state.blockers.is_empty();

            let search_str = format_search(&loc.search);
            let search_display = if search_str.is_empty() {
                "None".to_string()
            } else {
                search_str
            };

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
                                .gap_2()
                                .child(
                                    div()
                                        .text_color(theme.foreground)
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_size(px(12.0))
                                        .child("Navigation"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Current Path"),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.primary)
                                                .text_size(px(11.0))
                                                .child(loc.pathname.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Search Params"),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.foreground)
                                                .text_size(px(11.0))
                                                .child(search_display),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Can Go Back"),
                                        )
                                        .child(
                                            div()
                                                .text_color(if can_back {
                                                    theme.success
                                                } else {
                                                    theme.muted_foreground
                                                })
                                                .text_size(px(11.0))
                                                .child(can_back.to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Can Go Forward"),
                                        )
                                        .child(
                                            div()
                                                .text_color(if can_forward {
                                                    theme.success
                                                } else {
                                                    theme.muted_foreground
                                                })
                                                .text_size(px(11.0))
                                                .child(can_forward.to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Active Blockers"),
                                        )
                                        .child(
                                            div()
                                                .text_color(if has_blockers {
                                                    theme.warning
                                                } else {
                                                    theme.muted_foreground
                                                })
                                                .text_size(px(11.0))
                                                .child(blocker_count.to_string()),
                                        ),
                                ),
                        ),
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
                                .gap_2()
                                .child(
                                    div()
                                        .text_color(theme.foreground)
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_size(px(12.0))
                                        .child("Route Tree Statistics"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Total Routes"),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.foreground)
                                                .text_size(px(11.0))
                                                .child(total_routes.to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Layouts"),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.info)
                                                .text_size(px(11.0))
                                                .child(layout_count.to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Routes with Loaders"),
                                        )
                                        .child(
                                            div()
                                                .text_color(if loader_count > 0 {
                                                    theme.warning
                                                } else {
                                                    theme.muted_foreground
                                                })
                                                .text_size(px(11.0))
                                                .child(loader_count.to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child("Leaf Routes"),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.foreground)
                                                .text_size(px(11.0))
                                                .child((total_routes - layout_count).to_string()),
                                        ),
                                ),
                        ),
                );
        } else {
            container = container.child(
                div()
                    .text_color(theme.warning)
                    .child("No router state found"),
            );
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
            .on_action(cx.listener(|this, _: &SwitchToTab1, _window, cx| {
                this.set_selected_tab(DevtoolsTab::Routes, cx);
            }))
            .on_action(cx.listener(|this, _: &SwitchToTab2, _window, cx| {
                this.set_selected_tab(DevtoolsTab::Cache, cx);
            }))
            .on_action(cx.listener(|this, _: &SwitchToTab3, _window, cx| {
                this.set_selected_tab(DevtoolsTab::Timeline, cx);
            }))
            .on_action(cx.listener(|this, _: &SwitchToTab4, _window, cx| {
                this.set_selected_tab(DevtoolsTab::State, cx);
            }))
            .on_action(cx.listener(|this, _: &FocusTimelineSearch, window, cx| {
                this.set_selected_tab(DevtoolsTab::Timeline, cx);
                this.ensure_timeline_search(window, cx);
                if let Some(search) = &this.timeline_search {
                    search.update(cx, |state, cx| state.focus(window, cx));
                }
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
                        DevtoolsTab::Routes => {
                            self.render_routes_tab(window, cx).into_any_element()
                        }
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
