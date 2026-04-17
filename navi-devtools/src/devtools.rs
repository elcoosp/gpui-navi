//! Developer tools for Navi router.

use gpui::{
    AnyWindowHandle, App, Context, Div, Entity, EventEmitter, FocusHandle, FontWeight, Hsla,
    KeyBinding, MouseButton, Render, Size, StyledText, Subscription, TextStyle, WeakEntity, Window,
    actions, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size as ComponentSize, VirtualListScrollHandle,
    button::{Button, ButtonVariants},
    clipboard::Clipboard,
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    tab::{Tab, TabBar},
    table::{Column, ColumnSort, DataTable, TableDelegate, TableState},
    v_virtual_list,
};
use navi_router::{
    Navigator, RouterEvent, RouterState,
    event_bus::{self, TimedEvent},
};
use rs_query::QueryClient;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

actions!(
    devtools,
    [
        ToggleNaviDevtools,
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
    from_state: Option<String>,
    to_pathname: String,
    to_search: Option<String>,
    to_state: Option<String>,
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

            let from_state_str = from.as_ref().and_then(|l| {
                if l.state.is_null() {
                    None
                } else {
                    Some(l.state.to_string())
                }
            });
            let to_state_str = if to.state.is_null() {
                None
            } else {
                Some(to.state.to_string())
            };

            EventDetail {
                from_pathname: from.as_ref().map(|l| l.pathname.clone()),
                from_search: if from_search_str.is_empty() {
                    None
                } else {
                    Some(from_search_str)
                },
                from_state: from_state_str,
                to_pathname: to.pathname.clone(),
                to_search: if to_search_str.is_empty() {
                    None
                } else {
                    Some(to_search_str)
                },
                to_state: to_state_str,
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
// Cache Table Delegate
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct CacheEntryRow {
    key: String,
    age: Duration,
    is_stale: bool,
    #[allow(dead_code)]
    type_id: std::any::TypeId,
}

struct CacheTableDelegate {
    entries: Vec<CacheEntryRow>,
    columns: Vec<Column>,
    query_client: QueryClient,
    view: WeakEntity<DevtoolsState>,
}

impl CacheTableDelegate {
    fn new(
        entries: Vec<CacheEntryRow>,
        query_client: QueryClient,
        view: WeakEntity<DevtoolsState>,
    ) -> Self {
        let columns = vec![
            Column::new("key", "Key").width(px(200.)).resizable(true),
            Column::new("age", "Age").width(px(80.)).resizable(true),
            Column::new("stale", "Stale")
                .width(px(60.))
                .resizable(false),
            Column::new("actions", "")
                .width(px(80.))
                .resizable(false)
                .selectable(false),
        ];
        Self {
            entries,
            columns,
            query_client,
            view,
        }
    }
}

impl TableDelegate for CacheTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.entries.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        let mut col = self.columns[col_ix].clone();
        if col.key == "key" || col.key == "age" {
            col = col.sortable();
        }
        col
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        let col = &self.columns[col_ix];
        match col.key.as_ref() {
            "key" => {
                self.entries.sort_by(|a, b| {
                    if sort == ColumnSort::Ascending {
                        a.key.cmp(&b.key)
                    } else {
                        b.key.cmp(&a.key)
                    }
                });
            }
            "age" => {
                self.entries.sort_by(|a, b| {
                    if sort == ColumnSort::Ascending {
                        a.age.cmp(&b.age)
                    } else {
                        b.age.cmp(&a.age)
                    }
                });
            }
            _ => {}
        }
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let entry = &self.entries[row_ix];
        let col_key = &self.columns[col_ix].key;

        match col_key.as_ref() {
            "key" => div()
                .overflow_hidden()
                .text_ellipsis()
                .child(entry.key.clone())
                .into_any_element(),
            "age" => {
                let age_secs = entry.age.as_secs_f32();
                div()
                    .text_color(if age_secs > 60.0 {
                        cx.theme().warning
                    } else {
                        cx.theme().foreground
                    })
                    .child(format!("{:.1}s", age_secs))
                    .into_any_element()
            }
            "stale" => {
                if entry.is_stale {
                    div()
                        .text_color(cx.theme().warning)
                        .child("⚠ Stale")
                        .into_any_element()
                } else {
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child("✓")
                        .into_any_element()
                }
            }
            "actions" => {
                let key = entry.key.clone();
                let query_client = self.query_client.clone();
                let weak_view = self.view.clone();
                h_flex()
                    .gap_1()
                    .child(
                        Button::new(format!("invalidate-{}", key))
                            .icon(IconName::Loader)
                            .ghost()
                            .xsmall()
                            .tooltip("Invalidate")
                            .on_click({
                                let key = key.clone();
                                let query_client = query_client.clone();
                                let weak_view = weak_view.clone();
                                move |_, _, cx| {
                                    query_client.invalidate_queries(&rs_query::QueryKey::new(&key));
                                    if let Some(view) = weak_view.upgrade() {
                                        view.update(cx, |_: &mut DevtoolsState, cx| cx.notify());
                                    }
                                }
                            }),
                    )
                    .child(
                        Button::new(format!("remove-{}", key))
                            .icon(IconName::Delete)
                            .ghost()
                            .xsmall()
                            .tooltip("Remove")
                            .on_click({
                                let key = key.clone();
                                let query_client = query_client.clone();
                                let weak_view = weak_view.clone();
                                move |_, _, cx| {
                                    query_client.cache.remove(&key);
                                    if let Some(view) = weak_view.upgrade() {
                                        view.update(cx, |_: &mut DevtoolsState, cx| cx.notify());
                                    }
                                }
                            }),
                    )
                    .into_any_element()
            }
            _ => div().into_any_element(),
        }
    }
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
    filter_event_types: HashSet<RouterEventType>,
    focus_handle: FocusHandle,
    tree_search: Option<Entity<InputState>>,
    selected_event_detail: Option<EventDetail>,
    collapsed_route_nodes: HashSet<String>,
    route_test_params: Option<Entity<InputState>>,
    query_client: QueryClient,
    cache_table_state: Option<Entity<TableState<CacheTableDelegate>>>,
}

impl EventEmitter<()> for DevtoolsState {}

impl DevtoolsState {
    pub fn new(query_client: QueryClient, cx: &mut Context<Self>) -> Self {
        cx.bind_keys([KeyBinding::new(
            "cmd-shift-d",
            ToggleNaviDevtools,
            Some("Devtools"),
        )]);
        cx.bind_keys([KeyBinding::new(
            "ctrl-shift-d",
            ToggleNaviDevtools,
            Some("Devtools"),
        )]);
        cx.bind_keys([KeyBinding::new(
            "cmd-shift-tab",
            ToggleNaviDevtools,
            Some("Devtools"),
        )]);
        cx.bind_keys([KeyBinding::new(
            "ctrl-shift-tab",
            ToggleNaviDevtools,
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
            filter_event_types: HashSet::new(),
            focus_handle: cx.focus_handle(),
            tree_search: None,
            selected_event_detail: None,
            collapsed_route_nodes: HashSet::new(),
            route_test_params: None,
            query_client,
            cache_table_state: None,
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

    fn ensure_tree_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tree_search.is_none() {
            let state = cx.new(|cx| {
                let mut s = InputState::new(window, cx);
                s.set_placeholder("Filter routes...", window, cx);
                s
            });
            self.tree_search = Some(state);
        }
    }

    fn ensure_route_test_params(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.route_test_params.is_none() {
            let state = cx.new(|cx| {
                let mut s = InputState::new(window, cx);
                s.set_placeholder("/users/:id?tab=profile", window, cx);
                s
            });
            self.route_test_params = Some(state);
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
                let type_ok = self.filter_event_types.is_empty()
                    || self.filter_event_types.contains(&event_type);
                let text_ok =
                    query.is_empty() || format_event_text(&e.event).to_lowercase().contains(&query);
                type_ok && text_ok
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

    fn toggle_route_node(&mut self, node_id: String, cx: &mut Context<Self>) {
        if self.collapsed_route_nodes.contains(&node_id) {
            self.collapsed_route_nodes.remove(&node_id);
        } else {
            self.collapsed_route_nodes.insert(node_id);
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
        self.ensure_tree_search(window, cx);
        self.ensure_route_test_params(window, cx);

        let theme = cx.theme();
        let state = RouterState::try_global(cx);
        let mut container = div().gap_3().flex().flex_col();

        let window_handle_for_tree: AnyWindowHandle = window.window_handle();
        let tree_search_entity = self.tree_search.clone().unwrap();
        let route_test_entity = self.route_test_params.clone().unwrap();

        if let Some(state) = state {
            let loc = state.current_location();
            let search_str = format_search(&loc.search);
            let full_path = if search_str.is_empty() {
                loc.pathname.clone()
            } else {
                format!("{}{}", loc.pathname, search_str)
            };

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

            // Route Tester
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
                                    .child(Icon::new(IconName::SquareTerminal))
                                    .child("Route Tester"),
                            )
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .text_size(px(11.0))
                                    .child("Enter any path with parameters and query string"),
                            )
                            .child(
                                Input::new(&route_test_entity)
                                    .prefix(Icon::new(IconName::Globe))
                                    .cleanable(true)
                                    .small(),
                            )
                            .child(
                                Button::new("test-route-go")
                                    .label("Test Navigation")
                                    .primary()
                                    .small()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        let val = this
                                            .route_test_params
                                            .as_ref()
                                            .map(|i| i.read(cx).value().to_string())
                                            .unwrap_or_default();
                                        if !val.trim().is_empty() {
                                            let nav = Navigator::new(window.window_handle());
                                            nav.push(&val, cx);
                                        }
                                    })),
                            ),
                    ),
            );

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

            let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
            for (id, _, _, _, _, parent) in &node_infos {
                if let Some(p) = parent {
                    children_map.entry(p.clone()).or_default().push(id.clone());
                }
            }

            let root_ids: Vec<String> = node_infos
                .iter()
                .filter_map(|(id, _, _, _, _, parent)| {
                    if parent.is_none() {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();

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

            let mut node_infos_map: HashMap<String, (String, bool, bool, bool)> = HashMap::new();
            for (id, pattern, is_layout, is_index, has_loader, _) in &node_infos {
                node_infos_map.insert(
                    id.clone(),
                    (pattern.clone(), *is_layout, *is_index, *has_loader),
                );
            }

            container = container.child(
                Input::new(&tree_search_entity)
                    .prefix(Icon::new(IconName::Search))
                    .cleanable(true)
                    .small(),
            );

            let total_routes = node_infos.len();
            let layout_count = node_infos.iter().filter(|(_, _, l, _, _, _)| *l).count();
            let loader_count = node_infos.iter().filter(|(_, _, _, _, ld, _)| *ld).count();

            // Recursive tree rendering function
            fn render_node(
                id: &str,
                depth: usize,
                node_infos_map: &HashMap<String, (String, bool, bool, bool)>,
                children_map: &HashMap<String, Vec<String>>,
                collapsed: &HashSet<String>,
                matched_chain: &HashSet<String>,
                matched_leaf_id: Option<&str>,
                window_handle: AnyWindowHandle,
                cx: &mut Context<DevtoolsState>,
                window: &mut Window,
            ) -> Vec<Div> {
                let theme = cx.theme();
                let mut rows = Vec::new();
                let info = node_infos_map.get(id).unwrap();
                let pattern = &info.0;
                let is_layout = info.1;
                let is_index = info.2;
                let has_loader = info.3;
                let is_in_chain = matched_chain.contains(id);
                let is_leaf_match = matched_leaf_id == Some(id);
                let indent_px = px(16.0 * depth as f32);

                let has_children = children_map.get(id).map(|v| !v.is_empty()).unwrap_or(false);
                let is_collapsed = collapsed.contains(id);

                let mut row = div()
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
                    .hover(|style| style.bg(theme.secondary.opacity(0.5)));

                if has_children {
                    let id_clone = id.to_string();
                    let entity = cx.entity().clone();
                    row = row.child(
                        Button::new(format!("toggle-{}", id))
                            .icon(if is_collapsed {
                                IconName::ChevronRight
                            } else {
                                IconName::ChevronDown
                            })
                            .ghost()
                            .xsmall()
                            .on_click(move |_, _window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.toggle_route_node(id_clone.clone(), cx);
                                });
                            }),
                    );
                } else {
                    row = row.child(div().w(px(20.0)));
                }

                let pattern_clone = pattern.clone();
                let window_handle_clone = window_handle;
                row = row
                    .child(
                        div()
                            .min_w(px(90.0))
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
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, move |_ev, _window, cx| {
                                let nav = Navigator::new(window_handle_clone);
                                nav.push(&pattern_clone, cx);
                            })
                            .child(id.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.muted_foreground)
                            .text_size(px(11.0))
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, {
                                let pattern = pattern.clone();
                                let wh = window_handle;
                                move |_ev, _window, cx| {
                                    let nav = Navigator::new(wh);
                                    nav.push(&pattern, cx);
                                }
                            })
                            .child(pattern.clone()),
                    )
                    .when(is_layout || is_index || has_loader, |d| {
                        let mut tags = Vec::new();
                        if is_layout {
                            tags.push("layout");
                        }
                        if is_index {
                            tags.push("index");
                        }
                        if has_loader {
                            tags.push("loader");
                        }
                        d.child(div().flex().gap_1().children(tags.into_iter().map(|tag| {
                            div()
                                .px_1()
                                .rounded(px(2.0))
                                .bg(theme.muted_foreground.opacity(0.1))
                                .text_color(theme.muted_foreground)
                                .text_size(px(9.0))
                                .child(tag)
                        })))
                    });

                rows.push(row);

                if !is_collapsed
                    && let Some(children) = children_map.get(id) {
                        let mut sorted_children = children.clone();
                        sorted_children.sort();
                        for child_id in sorted_children {
                            rows.extend(render_node(
                                &child_id,
                                depth + 1,
                                node_infos_map,
                                children_map,
                                collapsed,
                                matched_chain,
                                matched_leaf_id,
                                window_handle,
                                cx,
                                window,
                            ));
                        }
                    }

                rows
            }

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
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .children(root_ids.iter().flat_map(|root_id| {
                            render_node(
                                root_id,
                                0,
                                &node_infos_map,
                                &children_map,
                                &self.collapsed_route_nodes,
                                &matched_chain,
                                matched_leaf_id.as_deref(),
                                window_handle_for_tree,
                                cx,
                                window,
                            )
                        })),
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

        let window_handle_for_jump: AnyWindowHandle = window.window_handle();

        let json_log = serde_json::to_string_pretty(&self.event_log)
            .unwrap_or_else(|_| "Failed to serialize log".to_string());

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
                                Button::new("filter-types")
                                    .label(if self.filter_event_types.is_empty() {
                                        "All Events".to_string()
                                    } else {
                                        format!("{} types", self.filter_event_types.len())
                                    })
                                    .ghost()
                                    .small()
                                    .dropdown_menu({
                                        let weak_self = weak_self.clone();
                                        move |menu, _window, cx| {
                                            let mut menu = menu;
                                            for event_type in [
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
                                                        this.read(cx)
                                                            .filter_event_types
                                                            .contains(&event_type)
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
                                                                    .update(cx, |this, cx| {
                                                                        if this
                                                                            .filter_event_types
                                                                            .contains(&event_type)
                                                                        {
                                                                            this.filter_event_types
                                                                                .remove(
                                                                                    &event_type,
                                                                                );
                                                                        } else {
                                                                            this.filter_event_types
                                                                                .insert(event_type);
                                                                        }
                                                                        cx.notify();
                                                                    })
                                                                    .ok();
                                                            }
                                                        }),
                                                );
                                            }
                                            menu = menu.item(
                                                PopupMenuItem::new("All Events").on_click({
                                                    let weak_self = weak_self.clone();
                                                    move |_, _window, cx| {
                                                        weak_self
                                                            .update(cx, |this, cx| {
                                                                this.filter_event_types.clear();
                                                                cx.notify();
                                                            })
                                                            .ok();
                                                    }
                                                }),
                                            );
                                            menu
                                        }
                                    }),
                            )
                            .child(
                                Clipboard::new("copy-all-timeline")
                                    .value(copy_all_text)
                                    .tooltip("Copy all events as text"),
                            )
                            .child(
                                Button::new("export-json-log")
                                    .icon(IconName::File)
                                    .ghost()
                                    .small()
                                    .tooltip("Export log as JSON")
                                    .on_click({
                                        let json_log = json_log.clone();
                                        move |_, _window, cx| {
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                                json_log.clone(),
                                            ));
                                        }
                                    }),
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

                                                let is_rendered = display.badge == "REN";
                                                let jump_path = if is_rendered {
                                                    let mut p = display.detail.to_pathname.clone();
                                                    if let Some(s) = &display.detail.to_search {
                                                        p.push_str(s);
                                                    }
                                                    Some(p)
                                                } else {
                                                    None
                                                };

                                                div()
                                                    .w(row_width)
                                                    .h(row_height)
                                                    .px_3()
                                                    .flex()
                                                    .items_center()
                                                    .gap_3()
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
                                                    .child(
                                                        div().overflow_hidden().child(styled_text),
                                                    )
                                                    .child(
                                                        Button::new(format!("select-event-{}", ix))
                                                            .icon(IconName::Info)
                                                            .ghost()
                                                            .xsmall()
                                                            .tooltip("Show event details")
                                                            .on_click({
                                                                let entity = entity_inner.clone();
                                                                let displays =
                                                                    displays_for_click.clone();
                                                                move |_, _window, cx| {
                                                                    if let Some(display) =
                                                                        displays.get(ix)
                                                                    {
                                                                        entity.update(
                                                                            cx,
                                                                            |state, cx| {
                                                                                state.select_event(
                                                                                    display
                                                                                        .detail
                                                                                        .clone(),
                                                                                    cx,
                                                                                );
                                                                            },
                                                                        );
                                                                    }
                                                                }
                                                            }),
                                                    )
                                                    .when(is_rendered, |d| {
                                                        let path = jump_path.clone().unwrap();
                                                        let window_handle =
                                                            window_handle_for_jump;
                                                        d.child(
                                                            Button::new(format!("jump-btn-{}", ix))
                                                                .icon(IconName::Play)
                                                                .ghost()
                                                                .xsmall()
                                                                .tooltip("Jump to this state")
                                                                .on_click(move |_, _window, cx| {
                                                                    let nav = Navigator::new(
                                                                        window_handle,
                                                                    );
                                                                    nav.push(&path, cx);
                                                                }),
                                                        )
                                                    })
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
                from_state: None,
                to_pathname: "?".to_string(),
                to_search: None,
                to_state: None,
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

        let state_block = |label: &str, value: String| -> Div {
            div()
                .pl(px(70.0))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_color(theme.muted_foreground)
                        .text_size(px(10.0))
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .p_2()
                        .bg(theme.background)
                        .rounded(px(4.0))
                        .text_color(theme.foreground)
                        .font_family("monospace")
                        .text_size(px(10.0))
                        .overflow_x_scrollbar()
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
            if let Some(state) = &detail.from_state {
                panel = panel.child(state_block("  State:", state.clone()));
            }
        } else {
            panel = panel.child(label_row("From:", "(initial navigation)".to_string()));
        }

        panel = panel.child(label_row("To:", detail.to_pathname.clone()));
        if let Some(search) = &detail.to_search {
            panel = panel.child(label_row("  Search:", search.clone()));
        }
        if let Some(state) = &detail.to_state {
            panel = panel.child(state_block("  State:", state.clone()));
        }

        panel
    }

    // -----------------------------------------------------------------------
    // Cache tab - structured table display
    // -----------------------------------------------------------------------

    fn render_cache_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Extract theme colors early to avoid borrow conflicts
        let theme = cx.theme();
        let info_color = theme.info;
        let muted_fg = theme.muted_foreground;
        let border_color = theme.border;

        let entries: Vec<_> = self.query_client.cache.iter().collect();

        // Build rows for the table
        let rows: Vec<CacheEntryRow> = entries
            .into_iter()
            .map(|entry| {
                let cached = entry.value();
                CacheEntryRow {
                    key: entry.key().clone(),
                    age: cached.fetched_at.elapsed(),
                    is_stale: cached.is_stale,
                    type_id: cached.type_id,
                }
            })
            .collect();

        let row_count = rows.len();

        // Create or update table state
        if self.cache_table_state.is_none() && !rows.is_empty() {
            let delegate =
                CacheTableDelegate::new(rows, self.query_client.clone(), cx.entity().downgrade());
            let state = cx.new(|cx| TableState::new(delegate, window, cx));
            self.cache_table_state = Some(state);
        } else if let Some(state) = self.cache_table_state.as_ref() {
            state.update(cx, |state, _| {
                let delegate = state.delegate_mut();
                delegate.entries = rows;
            });
        }

        div()
            .flex()
            .flex_col()
            .gap_2()
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
                            .child(Icon::new(IconName::Inbox))
                            .child(format!("rs-query Cache ({} entries)", row_count)),
                    )
                    .child(
                        Button::new("clear-cache")
                            .label("Clear All")
                            .ghost()
                            .small()
                            .on_click({
                                let query_client = self.query_client.clone();
                                let weak_self = cx.entity().downgrade();
                                move |_, _, cx| {
                                    query_client.cache.clear();
                                    if let Some(this) = weak_self.upgrade() {
                                        this.update(cx, |state, cx| {
                                            state.cache_table_state = None;
                                            cx.notify();
                                        });
                                    }
                                }
                            }),
                    ),
            )
            .child(if row_count == 0 {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_color(muted_fg)
                            .child("Cache is empty. No queries have been executed."),
                    )
                    .into_any_element()
            } else if let Some(state) = &self.cache_table_state {
                div()
                    .flex_1()
                    .border_1()
                    .border_color(border_color)
                    .rounded(px(6.0))
                    .overflow_hidden()
                    .child(
                        DataTable::new(state)
                            .bordered(false)
                            .stripe(true)
                            .with_size(ComponentSize::Small),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            })
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
            let is_blocked = state.is_blocked();

            let search_str = format_search(&loc.search);
            let search_display = if search_str.is_empty() {
                "None".to_string()
            } else {
                search_str
            };

            if is_blocked {
                container = container.child(
                    div()
                        .p_3()
                        .bg(theme.warning.opacity(0.15))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(theme.warning.opacity(0.5))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_color(theme.warning)
                                                .font_weight(FontWeight::BOLD)
                                                .text_size(px(12.0))
                                                .child("⚠ Navigation Blocked"),
                                        )
                                        .child(
                                            div()
                                                .text_color(theme.muted_foreground)
                                                .text_size(px(11.0))
                                                .child(format!(
                                                    "({} active blocker{}))",
                                                    blocker_count,
                                                    if blocker_count > 1 { "s" } else { "" }
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            Button::new("blocker-proceed")
                                                .label("Proceed")
                                                .primary()
                                                .small()
                                                .on_click(cx.listener(|_, _, _, cx| {
                                                    RouterState::update(cx, |state, cx| {
                                                        state.proceed(cx);
                                                    });
                                                })),
                                        )
                                        .child(
                                            Button::new("blocker-cancel")
                                                .label("Cancel Pending")
                                                .ghost()
                                                .small()
                                                .on_click(cx.listener(|_, _, _, cx| {
                                                    RouterState::update(cx, |state, _cx| {
                                                        state.reset_block();
                                                    });
                                                })),
                                        ),
                                ),
                        ),
                );
            }

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
                                                .child(if is_blocked {
                                                    "Pending".to_string()
                                                } else {
                                                    blocker_count.to_string()
                                                }),
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
                .on_action(cx.listener(|this, _: &ToggleNaviDevtools, window, cx| {
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
            .bg(theme.background.opacity(0.9))
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
            .on_action(cx.listener(|this, _: &ToggleNaviDevtools, window, cx| {
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
                    .bg(theme.secondary.opacity(0.95))
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
                        DevtoolsTab::Cache => self.render_cache_tab(window, cx).into_any_element(),
                        DevtoolsTab::Timeline => {
                            self.render_timeline_tab(window, cx).into_any_element()
                        }
                        DevtoolsTab::State => self.render_state_tab(cx).into_any_element(),
                    }),
            )
            .into_any_element()
    }
}
