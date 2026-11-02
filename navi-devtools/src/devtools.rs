//! Developer tools for the Navi router — framework-agnostic edition.
//!
//! The panel is a pure [`DevtoolsModel`] whose `render` produces
//! `quoin_ui::Node` trees; the host application materializes them through a
//! quoin adapter. Every capability of the classic GPUI-native devtools is
//! preserved: timeline with delta column, search highlighting and event
//! filtering; the routes tree with matched-chain highlighting, metadata
//! badges and a route tester; the rs-query cache table with sort,
//! invalidate and remove actions; the router state inspector; the FAB /
//! expanded panel toggle; and the full keyboard shortcut set.

use navi_router::{
    Navigator, NotFoundMode, RouterEvent, RouterHandle,
    event_bus::{self, TimedEvent},
};
use quoin_ui::node::{NodeEventCtx, StyleCmd};
use quoin_ui::{
    InputValues, Node, NodeKeySpec, StyledRun, TextColor, ThemeToken, button, clipboard,
    data_table, input, tab_bar, virtual_list,
};
use rs_query::QueryClient;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

pub use crate::timeline::{DevtoolsEvent, LoggedEvent};

/// Stable node ids used by the panel; the adapter keys widget state by
/// them, and handlers read live input values through them.
pub mod ids {
    /// Timeline search input.
    pub const TIMELINE_SEARCH: &str = "devtools-timeline-search";
    /// Routes tree filter input.
    pub const TREE_SEARCH: &str = "devtools-tree-search";
    /// Route tester path input.
    pub const ROUTE_TEST: &str = "devtools-route-test";
    /// The devtools panel / FAB root (shared focus handle).
    pub const ROOT: &str = "navi-devtools";
    /// The timeline virtual list.
    pub const TIMELINE_LIST: &str = "timeline-list";
    /// The cache data table.
    pub const CACHE_TABLE: &str = "devtools-cache-table";
}

/// Coarse classification of router events (timeline filter entries).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RouterEventType {
    BeforeNavigate,
    BeforeLoad,
    Load,
    BeforeRouteMount,
    Resolved,
    Rendered,
}

impl RouterEventType {
    /// Human label used in the filter menu.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::BeforeNavigate => "BeforeNavigate",
            Self::BeforeLoad => "BeforeLoad",
            Self::Load => "Load",
            Self::BeforeRouteMount => "BeforeRouteMount",
            Self::Resolved => "Resolved",
            Self::Rendered => "Rendered",
        }
    }

    /// All filterable event types, in menu order.
    #[must_use]
    pub fn all() -> [Self; 6] {
        [
            Self::BeforeNavigate,
            Self::BeforeLoad,
            Self::Load,
            Self::BeforeRouteMount,
            Self::Resolved,
            Self::Rendered,
        ]
    }

    /// Classifies a router event.
    #[must_use]
    pub fn from_event(event: &RouterEvent) -> Self {
        match event {
            RouterEvent::BeforeNavigate { .. } => Self::BeforeNavigate,
            RouterEvent::BeforeLoad { .. } => Self::BeforeLoad,
            RouterEvent::Load { .. } => Self::Load,
            RouterEvent::BeforeRouteMount { .. } => Self::BeforeRouteMount,
            RouterEvent::Resolved { .. } => Self::Resolved,
            RouterEvent::Rendered { .. } => Self::Rendered,
        }
    }
}

/// The devtools panel tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevtoolsTab {
    Routes,
    Cache,
    Timeline,
    State,
}

/// Extracted detail payload of a router event (event detail panel).
#[derive(Clone, Debug, PartialEq)]
pub struct EventDetail {
    pub from_pathname: Option<String>,
    pub from_search: Option<String>,
    pub from_state: Option<String>,
    pub to_pathname: String,
    pub to_search: Option<String>,
    pub to_state: Option<String>,
}

/// Formats a search JSON value as a query-string fragment.
#[must_use]
pub fn format_search(search: &serde_json::Value) -> String {
    if let serde_json::Value::Object(map) = search {
        if map.is_empty() {
            return String::new();
        }
        let pairs: Vec<String> = map
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                format!("{k}={val}")
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

/// Extracts an [`EventDetail`] from a router event.
#[must_use]
pub fn extract_event_detail(event: &RouterEvent) -> EventDetail {
    fn loc_parts(
        loc: Option<&navi_router::Location>,
    ) -> (Option<String>, Option<String>, Option<String>) {
        match loc {
            Some(location) => {
                let search = format_search(&location.search);
                let search = if search.is_empty() {
                    None
                } else {
                    Some(search)
                };
                let state = if location.state.is_null() {
                    None
                } else {
                    Some(location.state.to_string())
                };
                (Some(location.pathname.clone()), search, state)
            }
            None => (None, None, None),
        }
    }

    match event {
        RouterEvent::BeforeNavigate { from, to }
        | RouterEvent::BeforeLoad { from, to }
        | RouterEvent::Load { from, to }
        | RouterEvent::BeforeRouteMount { from, to }
        | RouterEvent::Resolved { from, to }
        | RouterEvent::Rendered { from, to } => {
            let (from_pathname, from_search, from_state) = loc_parts(from.as_ref());
            EventDetail {
                from_pathname,
                from_search,
                from_state,
                to_pathname: to.pathname.clone(),
                to_search: {
                    let s = format_search(&to.search);
                    (!s.is_empty()).then_some(s)
                },
                to_state: (!to.state.is_null()).then(|| to.state.to_string()),
            }
        }
    }
}

/// One timeline row ready for display.
#[derive(Clone, Debug)]
pub struct EventDisplay {
    pub timestamp_str: String,
    pub badge: &'static str,
    pub badge_color: TextColor,
    pub text: String,
    pub detail: EventDetail,
}

/// Theme tokens used by the timeline display.
#[derive(Clone, Copy)]
pub struct EventColors {
    pub foreground: ThemeToken,
    pub primary: ThemeToken,
    pub success: ThemeToken,
    pub warning: ThemeToken,
    pub info: ThemeToken,
}

/// Formats the human text for a router event (timeline + search matching).
#[must_use]
pub fn format_event_text(event: &RouterEvent) -> String {
    match event {
        RouterEvent::BeforeNavigate { from, to } => {
            let from_path = from.as_ref().map_or_else(
                || "?".to_string(),
                |l| format!("{}{}", l.pathname, format_search(&l.search)),
            );
            let to_path = format!("{}{}", to.pathname, format_search(&to.search));
            format!("{from_path} → {to_path}")
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

/// Builds a display row for a timed event.
#[must_use]
pub fn build_event_display(
    event: &RouterEvent,
    timestamp_str: String,
    colors: &EventColors,
) -> EventDisplay {
    let (badge, badge_color) = match event {
        RouterEvent::BeforeNavigate { .. } => ("NAV", TextColor::Token(colors.primary)),
        RouterEvent::BeforeLoad { .. } => ("BLD", TextColor::Token(colors.warning)),
        RouterEvent::Load { .. } => ("LOAD", TextColor::Token(colors.warning)),
        RouterEvent::BeforeRouteMount { .. } => ("MNT", TextColor::Token(colors.info)),
        RouterEvent::Resolved { .. } => ("OK", TextColor::Token(colors.success)),
        RouterEvent::Rendered { .. } => ("REN", TextColor::Token(colors.info)),
    };
    EventDisplay {
        timestamp_str,
        badge,
        badge_color,
        text: format_event_text(event),
        detail: extract_event_detail(event),
    }
}

/// Enhanced node info for the routes tree (meta, timings, before-load).
#[derive(Clone)]
struct EnhancedNodeInfo {
    pattern: String,
    is_layout: bool,
    is_index: bool,
    has_loader: bool,
    stale_time: Option<Duration>,
    gc_time: Option<Duration>,
    meta: HashMap<String, serde_json::Value>,
    before_load: bool,
}

/// One cache-table row.
#[derive(Clone)]
struct CacheEntryRow {
    key: String,
    age: Duration,
    is_stale: bool,
}

/// Shared model handle used by event handlers.
pub type SharedModel = Rc<RefCell<DevtoolsModel>>;

/// The framework-agnostic devtools panel model.
pub struct DevtoolsModel {
    expanded: bool,
    selected_tab: DevtoolsTab,
    event_log: Vec<TimedEvent>,
    last_log_len: usize,
    highlight_new_count: usize,
    filter_event_types: HashSet<RouterEventType>,
    selected_event_detail: Option<EventDetail>,
    collapsed_route_nodes: HashSet<String>,
    query_client: QueryClient,
    /// Active cache sort: `(column key, ascending)`.
    cache_sort: Option<(String, bool)>,
    /// Desired timeline-search override (tour hooks); cleared by edits.
    timeline_override: Option<String>,
    /// Input id to focus after the next render.
    focus_request: Option<String>,
    /// Repaint hook (host refresh).
    refresh: Rc<dyn Fn()>,
    /// Timeline events logged by the app (loader/navigation records).
    timeline_events: Vec<LoggedEvent>,
}

impl DevtoolsModel {
    /// Creates the model. `refresh` is the repaint hook (the router host
    /// refresh or an equivalent framework callback).
    #[must_use]
    pub fn new(query_client: QueryClient, refresh: Rc<dyn Fn()>) -> Self {
        Self {
            expanded: true,
            selected_tab: DevtoolsTab::Timeline,
            event_log: Vec::new(),
            last_log_len: 0,
            highlight_new_count: 0,
            filter_event_types: HashSet::new(),
            selected_event_detail: None,
            collapsed_route_nodes: HashSet::new(),
            query_client,
            cache_sort: None,
            timeline_override: None,
            focus_request: None,
            refresh,
            timeline_events: Vec::new(),
        }
    }

    /// Requests a repaint through the host hook.
    pub fn notify(&self) {
        (self.refresh)();
    }

    /// The keyboard shortcut set (framework-agnostic).
    #[must_use]
    pub fn keybindings(&self) -> Vec<NodeKeySpec> {
        let mut bindings = Vec::new();
        for keystroke in [
            "cmd-shift-d",
            "ctrl-shift-d",
            "cmd-shift-tab",
            "ctrl-shift-tab",
        ] {
            bindings.push(NodeKeySpec {
                keystroke: keystroke.to_string(),
                action: "toggle-devtools".to_string(),
                context: "Devtools".to_string(),
            });
        }
        for (keystroke, action) in [
            ("cmd-1", "switch-tab-1"),
            ("ctrl-1", "switch-tab-1"),
            ("cmd-2", "switch-tab-2"),
            ("ctrl-2", "switch-tab-2"),
            ("cmd-3", "switch-tab-3"),
            ("ctrl-3", "switch-tab-3"),
            ("cmd-4", "switch-tab-4"),
            ("ctrl-4", "switch-tab-4"),
            ("cmd-f", "focus-timeline-search"),
            ("ctrl-f", "focus-timeline-search"),
        ] {
            bindings.push(NodeKeySpec {
                keystroke: keystroke.to_string(),
                action: action.to_string(),
                context: "Devtools".to_string(),
            });
        }
        bindings
    }

    /// Screenshot-tour hook: force the active tab.
    pub fn tour_set_tab(&mut self, tab: DevtoolsTab) {
        // Selecting the already-selected tab is a no-op (no notify, no
        // re-render), matching the classic panel's behavior.
        if self.selected_tab != tab {
            self.set_selected_tab(tab);
        }
    }

    /// Screenshot-tour hook: replace the timeline search text.
    pub fn tour_set_timeline_search(&mut self, query: &str) {
        self.refresh_log();
        self.timeline_override = Some(query.to_string());
        self.highlight_new_count = 0;
        self.notify();
    }

    /// Screenshot-tour hook: toggle the expanded/collapsed (FAB) state.
    pub fn tour_toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
        self.refresh_log();
        self.notify();
    }

    /// Logs a timeline event (loader/navigation lifecycle record).
    pub fn log_event(&mut self, event: DevtoolsEvent) {
        self.timeline_events.push(LoggedEvent::new(event));
        self.notify();
    }

    /// The recorded timeline events.
    #[must_use]
    pub fn timeline_events(&self) -> &[LoggedEvent] {
        &self.timeline_events
    }

    fn refresh_log(&mut self) {
        let new_log = event_bus::get_event_log();
        let new_len = new_log.len();
        let new_events_count = new_len.saturating_sub(self.last_log_len);
        self.highlight_new_count = new_events_count;
        self.last_log_len = new_len;
        self.event_log = new_log;
    }

    fn filtered_events(&self, query: &str) -> Vec<TimedEvent> {
        let query = query.to_lowercase();
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

    fn set_selected_tab(&mut self, tab: DevtoolsTab) {
        self.selected_tab = tab;
        self.selected_event_detail = None;
        self.notify();
    }

    fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
        self.notify();
    }

    fn tab_index(&self) -> usize {
        match self.selected_tab {
            DevtoolsTab::Routes => 0,
            DevtoolsTab::Cache => 1,
            DevtoolsTab::Timeline => 2,
            DevtoolsTab::State => 3,
        }
    }

    fn tab_from_index(idx: usize) -> DevtoolsTab {
        // Mirror of [`Self::tab_index`]: the TabBar reports 0-based indices
        // in child order (Routes, Cache, Timeline, State).
        match idx {
            0 => DevtoolsTab::Routes,
            1 => DevtoolsTab::Cache,
            2 => DevtoolsTab::Timeline,
            _ => DevtoolsTab::State,
        }
    }

    fn select_event(&mut self, detail: Option<EventDetail>) {
        self.selected_event_detail = detail;
        self.notify();
    }

    fn toggle_route_node(&mut self, node_id: String) {
        if self.collapsed_route_nodes.contains(&node_id) {
            self.collapsed_route_nodes.remove(&node_id);
        } else {
            self.collapsed_route_nodes.insert(node_id);
        }
        self.notify();
    }

    /// The input to focus after the current render (one-shot).
    pub fn take_focus_request(&mut self) -> Option<String> {
        self.focus_request.take()
    }

    // -------------------------------------------------------------------
    // Rendering
    // -------------------------------------------------------------------

    /// Renders the panel (or the FAB when collapsed) as a node tree.
    ///
    /// `me` is the shared handle baked into event handlers; `inputs` reads
    /// live native-input values; `viewport` is the host window's logical
    /// size (panel clamping).
    #[must_use]
    pub fn render(me: &SharedModel, inputs: &dyn InputValues, viewport: (f32, f32)) -> Node {
        {
            let mut this = me.borrow_mut();
            this.refresh_log();
            if !this.expanded {
                return render_fab(me);
            }
        }

        render_panel(me, inputs, viewport)
    }
}

fn action_handler(
    me: &SharedModel,
    f: impl Fn(&mut DevtoolsModel, &mut NodeEventCtx) + 'static,
) -> impl Fn(&mut NodeEventCtx) + 'static {
    let me = Rc::clone(me);
    move |ctx| {
        f(&mut me.borrow_mut(), ctx);
    }
}

fn render_fab(me: &SharedModel) -> Node {
    let toggle = action_handler(me, |model, _ctx| {
        model.expanded = !model.expanded;
        model.notify();
    });
    Node::div()
        .id(ids::ROOT)
        .styles([
            StyleCmd::Absolute,
            StyleCmd::Bottom(12.0),
            StyleCmd::Right(12.0),
        ])
        .key_context("Devtools")
        .track_focus()
        .on_action("toggle-devtools", toggle)
        .child({
            let toggle2 = action_handler(me, |model, _ctx| {
                model.expanded = !model.expanded;
                model.notify();
            });
            button("devtools-toggle")
                .icon("info")
                .ghost()
                .styles([
                    StyleCmd::RoundedFull,
                    StyleCmd::Width(40.0),
                    StyleCmd::Height(40.0),
                    StyleCmd::ShadowMd,
                ])
                .on_click(move |ctx| toggle2(ctx))
        })
        .build()
}

fn render_panel(me: &SharedModel, inputs: &dyn InputValues, viewport: (f32, f32)) -> Node {
    let panel_width = 550.0_f32.min(viewport.0 - 20.0);
    let panel_height = 450.0_f32.min(viewport.1 - 20.0);

    let tab_bar_node = tab_bar("devtools-tabs")
        .tab("Routes", Some("folder".to_string()))
        .tab("Cache", Some("inbox".to_string()))
        .tab("Timeline", Some("calendar".to_string()))
        .tab("State", Some("settings".to_string()))
        .selected(me.borrow().tab_index())
        .on_select({
            let me = Rc::clone(me);
            move |index| {
                me.borrow_mut()
                    .set_selected_tab(DevtoolsModel::tab_from_index(index));
            }
        });

    let body = match me.borrow().selected_tab {
        DevtoolsTab::Routes => render_routes_tab(me, inputs),
        DevtoolsTab::Cache => render_cache_tab(me),
        DevtoolsTab::Timeline => render_timeline_tab(me, inputs),
        DevtoolsTab::State => render_state_tab(),
    };

    // Header row: icon + title + close button.
    let header = Node::div()
        .styles([
            StyleCmd::Flex,
            StyleCmd::ItemsCenter,
            StyleCmd::JustifyBetween,
            StyleCmd::PaddingX(8.0),
            StyleCmd::PaddingY(4.0),
            StyleCmd::Background(TextColor::TokenWithOpacity(ThemeToken::Secondary, 0.95)),
            StyleCmd::RoundedTopLeft(8.0),
            StyleCmd::BorderBottom,
            StyleCmd::BorderColor(TextColor::Token(ThemeToken::Border)),
        ])
        .child(
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::Gap(4.0)])
                .child(
                    Node::div()
                        .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::Gap(4.0)])
                        .child(Node::icon("info"))
                        .child(" Devtools"),
                )
                .child(
                    button("close-devtools")
                        .icon("close")
                        .ghost()
                        .small()
                        .on_click(action_handler(me, |model, _ctx| {
                            model.toggle_expanded();
                        })),
                ),
        );

    let mut panel = Node::div()
        .id(ids::ROOT)
        .styles([
            StyleCmd::Absolute,
            StyleCmd::Bottom(0.0),
            StyleCmd::Right(0.0),
            StyleCmd::Width(panel_width),
            StyleCmd::Height(panel_height),
            StyleCmd::Background(TextColor::TokenWithOpacity(ThemeToken::Background, 0.9)),
            StyleCmd::TextColor(TextColor::Token(ThemeToken::Foreground)),
            StyleCmd::BorderAll,
            StyleCmd::BorderColor(TextColor::Token(ThemeToken::Border)),
            StyleCmd::RoundedTopLeft(8.0),
            StyleCmd::ShadowLg,
            StyleCmd::Flex,
            StyleCmd::FlexCol,
            StyleCmd::OverflowHidden,
        ])
        .key_context("Devtools")
        .track_focus()
        .on_action(
            "toggle-devtools",
            action_handler(me, |model, _ctx| {
                model.toggle_expanded();
            }),
        )
        .on_action(
            "switch-tab-1",
            action_handler(me, |model, _ctx| {
                model.set_selected_tab(DevtoolsTab::Routes);
            }),
        )
        .on_action(
            "switch-tab-2",
            action_handler(me, |model, _ctx| {
                model.set_selected_tab(DevtoolsTab::Cache);
            }),
        )
        .on_action(
            "switch-tab-3",
            action_handler(me, |model, _ctx| {
                model.set_selected_tab(DevtoolsTab::Timeline);
            }),
        )
        .on_action(
            "switch-tab-4",
            action_handler(me, |model, _ctx| {
                model.set_selected_tab(DevtoolsTab::State);
            }),
        )
        .on_action(
            "focus-timeline-search",
            action_handler(me, |model, _ctx| {
                model.set_selected_tab(DevtoolsTab::Timeline);
                model.focus_request = Some(ids::TIMELINE_SEARCH.to_string());
            }),
        )
        .child(header)
        .child(tab_bar_node.build())
        .child(
            Node::div()
                .styles([
                    StyleCmd::Flex1,
                    StyleCmd::Padding(12.0),
                    StyleCmd::OverflowYScroll,
                ])
                .child(body),
        );
    let _ = &mut panel;
    panel.build()
}

// ---------------------------------------------------------------------------
// Routes tab
// ---------------------------------------------------------------------------

fn section_card(children: Vec<Node>) -> Node {
    Node::div()
        .styles([
            StyleCmd::Padding(12.0),
            StyleCmd::Background(TextColor::Token(ThemeToken::Secondary)),
            StyleCmd::Rounded(6.0),
            StyleCmd::BorderAll,
            StyleCmd::BorderColor(TextColor::Token(ThemeToken::Border)),
        ])
        .children(children)
        .build()
}

fn label_row(label: &str, value: String, value_color: TextColor) -> Node {
    Node::div()
        .styles([StyleCmd::Flex, StyleCmd::JustifyBetween])
        .child(
            Node::div()
                .styles([
                    StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                    StyleCmd::TextSize(11.0),
                ])
                .child(label),
        )
        .child(
            Node::div()
                .styles([StyleCmd::TextColor(value_color), StyleCmd::TextSize(11.0)])
                .child(value),
        )
        .build()
}

fn section_title(text: &str, icon: &str, color: ThemeToken) -> Node {
    Node::div()
        .styles([
            StyleCmd::Flex,
            StyleCmd::ItemsCenter,
            StyleCmd::Gap(4.0),
            StyleCmd::TextColor(TextColor::Token(color)),
            StyleCmd::FontWeight(quoin_ui::NodeFontWeight::Medium),
        ])
        .child(Node::icon(icon))
        .child(text)
        .build()
}

fn render_routes_tab(me: &SharedModel, inputs: &dyn InputValues) -> Node {
    let mut container =
        Node::div().styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(12.0)]);

    let Some(handle) = RouterHandle::try_current() else {
        return container
            .child(
                Node::div()
                    .styles([StyleCmd::TextColor(TextColor::Token(ThemeToken::Warning))])
                    .child("No router state found"),
            )
            .build();
    };

    let state = handle.read();
    let loc = state.current_location();
    let search_str = format_search(&loc.search);
    let full_path = if search_str.is_empty() {
        loc.pathname.clone()
    } else {
        format!("{}{}", loc.pathname, search_str)
    };

    // Current location card.
    container = container.child(section_card(vec![
        Node::div()
            .styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(4.0)])
            .child(
                Node::div()
                    .styles([
                        StyleCmd::Flex,
                        StyleCmd::ItemsCenter,
                        StyleCmd::JustifyBetween,
                    ])
                    .child(section_title(
                        "Current Location",
                        "map",
                        ThemeToken::Primary,
                    ))
                    .child(
                        clipboard("copy-current-path")
                            .value(full_path)
                            .tooltip("Copy path")
                            .build(),
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
            ))
            .build(),
    ]));

    // Route tester card.
    let route_test_value = inputs.value(ids::ROUTE_TEST);
    container = container.child(section_card(vec![
        Node::div()
            .styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(8.0)])
            .child(section_title(
                "Route Tester",
                "square-terminal",
                ThemeToken::Info,
            ))
            .child(
                Node::div()
                    .styles([
                        StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                        StyleCmd::TextSize(11.0),
                    ])
                    .child("Enter any path with parameters and query string"),
            )
            .child(
                input(ids::ROUTE_TEST)
                    .value(route_test_value)
                    .placeholder("/users/:id?tab=profile")
                    .prefix_icon("globe")
                    .cleanable()
                    .small()
                    .build(),
            )
            .child(
                button("test-route-go")
                    .label("Test Navigation")
                    .primary()
                    .small()
                    .on_click(action_handler(me, |model, ctx| {
                        let val = ctx.input_value(ids::ROUTE_TEST).unwrap_or_default();
                        if !val.trim().is_empty() {
                            let _ = model;
                            Navigator::new().push(val);
                        }
                    })),
            )
            .build(),
    ]));

    // Route tree card.
    let matched_info = state
        .current_match
        .as_ref()
        .map(|(params, node)| (node.id.clone(), format!("{params:?}")));

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

    let mut parent_of: HashMap<String, String> = HashMap::new();
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for (id, _, _, _, _, parent) in &node_infos {
        if let Some(p) = parent {
            parent_of.insert(id.clone(), p.clone());
            children_map.entry(p.clone()).or_default().push(id.clone());
        }
    }

    let root_ids: Vec<String> = node_infos
        .iter()
        .filter(|(_, _, _, _, _, parent)| parent.is_none())
        .map(|(id, _, _, _, _, _)| id.clone())
        .collect();

    let matched_leaf_id: Option<String> = matched_info.as_ref().map(|(id, _)| id.clone());
    let matched_chain: HashSet<String> =
        matched_leaf_id
            .as_ref()
            .map_or_else(HashSet::new, |leaf_id| {
                let mut chain = HashSet::new();
                let mut cur: &str = leaf_id.as_str();
                loop {
                    chain.insert(cur.to_string());
                    cur = match parent_of.get(cur) {
                        Some(p) => p.as_str(),
                        None => break,
                    };
                }
                chain
            });

    let mut node_details: HashMap<String, EnhancedNodeInfo> = HashMap::new();
    for node in state.route_tree.all_nodes() {
        node_details.insert(
            node.id.clone(),
            EnhancedNodeInfo {
                pattern: node.pattern.raw.clone(),
                is_layout: node.is_layout,
                is_index: node.is_index,
                has_loader: node.has_loader,
                stale_time: node.loader_stale_time,
                gc_time: node.loader_gc_time,
                meta: node.meta.clone(),
                before_load: node.before_load.is_some(),
            },
        );
    }

    let tree_query = inputs.value(ids::TREE_SEARCH);
    let tree_query_lower = tree_query.to_lowercase();

    let total_routes = node_infos.len();
    let layout_count = node_infos.iter().filter(|(_, _, l, _, _, _)| *l).count();
    let loader_count = node_infos.iter().filter(|(_, _, _, _, ld, _)| *ld).count();

    container = container.child(
        input(ids::TREE_SEARCH)
            .value(inputs.value(ids::TREE_SEARCH))
            .placeholder("Filter routes...")
            .prefix_icon("search")
            .cleanable()
            .small()
            .build(),
    );

    container = container
        .child(
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::JustifyBetween])
                .child(section_title("Route Tree", "folder-open", ThemeToken::Info))
                .child(
                    Node::div()
                        .styles([
                            StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                            StyleCmd::TextSize(11.0),
                        ])
                        .child(format!(
                            "{total_routes} routes · {layout_count} layouts · {loader_count} with loaders"
                        )),
                )
                .build(),
        )
        .child(
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::FlexCol])
                .children(root_ids.iter().flat_map(|root_id| {
                    render_node(
                        root_id,
                        0,
                        &node_details,
                        &children_map,
                        &me.borrow().collapsed_route_nodes,
                        &matched_chain,
                        matched_leaf_id.as_deref(),
                        &tree_query_lower,
                        me,
                    )
                }))
                .build(),
        );

    drop(state);
    container.build()
}

/// Renders one routes-tree row (and its children when expanded).
#[allow(clippy::too_many_arguments)]
fn render_node(
    id: &str,
    depth: usize,
    node_details: &HashMap<String, EnhancedNodeInfo>,
    children_map: &HashMap<String, Vec<String>>,
    collapsed: &HashSet<String>,
    matched_chain: &HashSet<String>,
    matched_leaf_id: Option<&str>,
    tree_query_lower: &str,
    me: &SharedModel,
) -> Vec<Node> {
    let Some(info) = node_details.get(id) else {
        return Vec::new();
    };
    let pattern = &info.pattern;
    let is_layout = info.is_layout;
    let is_index = info.is_index;
    let has_loader = info.has_loader;
    let is_in_chain = matched_chain.contains(id);
    let is_leaf_match = matched_leaf_id == Some(id);
    let indent_px = 16.0 * f32::from(u16::try_from(depth).unwrap_or(u16::MAX));

    // Optional live filter (devtools tree search).
    if !tree_query_lower.is_empty()
        && !id.to_lowercase().contains(tree_query_lower)
        && !pattern.to_lowercase().contains(tree_query_lower)
    {
        // Keep rows whose children match so the tree stays navigable.
        let children_keep = children_map.get(id).is_some_and(|kids| {
            kids.iter().any(|kid| {
                kid.to_lowercase().contains(tree_query_lower)
                    || node_details
                        .get(kid)
                        .is_some_and(|i| i.pattern.to_lowercase().contains(tree_query_lower))
            })
        });
        if !children_keep {
            return Vec::new();
        }
    }

    let has_children = children_map.get(id).is_some_and(|v| !v.is_empty());
    let is_collapsed = collapsed.contains(id);

    let mut row = Node::div()
        .styles([
            StyleCmd::Flex,
            StyleCmd::ItemsCenter,
            StyleCmd::Gap(8.0),
            StyleCmd::PaddingLeft(indent_px),
            StyleCmd::PaddingRight(8.0),
            StyleCmd::PaddingY(3.0),
            StyleCmd::Rounded(4.0),
            StyleCmd::Hover(vec![StyleCmd::Background(TextColor::TokenWithOpacity(
                ThemeToken::Secondary,
                0.5,
            ))]),
        ])
        .styles({
            let mut cmds = Vec::new();
            if is_in_chain {
                cmds.push(StyleCmd::Background(TextColor::TokenWithOpacity(
                    if is_leaf_match {
                        ThemeToken::Primary
                    } else {
                        ThemeToken::Success
                    },
                    if is_leaf_match { 0.2 } else { 0.1 },
                )));
            }
            cmds
        });

    if has_children {
        let id_clone = id.to_string();
        row = row.child(
            button(format!("toggle-{id}"))
                .icon(if is_collapsed {
                    "chevron-right"
                } else {
                    "chevron-down"
                })
                .ghost()
                .xsmall()
                .on_click(action_handler(me, move |model, _ctx| {
                    model.toggle_route_node(id_clone.clone());
                })),
        );
    } else {
        row = row.child(Node::div().styles([StyleCmd::Width(20.0)]).build());
    }

    let pattern_color = if is_leaf_match {
        TextColor::Token(ThemeToken::Primary)
    } else if is_in_chain {
        TextColor::Token(ThemeToken::Success)
    } else {
        TextColor::Token(ThemeToken::Foreground)
    };
    let pattern_clone = pattern.clone();
    row = row
        .child(
            Node::div()
                .styles([
                    StyleCmd::MinWidth(90.0),
                    StyleCmd::TextColor(pattern_color),
                    StyleCmd::TextSize(11.0),
                    StyleCmd::FontWeight(if is_leaf_match {
                        quoin_ui::NodeFontWeight::Medium
                    } else {
                        quoin_ui::NodeFontWeight::Normal
                    }),
                    StyleCmd::CursorPointer,
                ])
                .on_click({
                    let pattern_clone = pattern_clone.clone();
                    move |_ctx: &mut NodeEventCtx| {
                        Navigator::new().push(pattern_clone.clone());
                    }
                })
                .child(id),
        )
        .child(
            Node::div()
                .styles([
                    StyleCmd::Flex1,
                    StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                    StyleCmd::TextSize(11.0),
                    StyleCmd::CursorPointer,
                ])
                .on_click({
                    let pattern_clone = pattern_clone.clone();
                    move |_ctx: &mut NodeEventCtx| {
                        Navigator::new().push(pattern_clone.clone());
                    }
                })
                .child(pattern.clone()),
        );

    if is_layout || is_index || has_loader {
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
        row = row.child(
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::Gap(4.0)])
                .children(tags.into_iter().map(|tag| {
                    Node::div()
                        .styles([
                            StyleCmd::PaddingX(4.0),
                            StyleCmd::Rounded(2.0),
                            StyleCmd::Background(TextColor::TokenWithOpacity(
                                ThemeToken::MutedForeground,
                                0.1,
                            )),
                            StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                            StyleCmd::TextSize(9.0),
                        ])
                        .child(tag)
                        .build()
                }))
                .build(),
        );
    }

    // Enhanced details: meta, stale/gc times, before_load.
    if !info.meta.is_empty() {
        let meta_str = info
            .meta
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(", ");
        row = row.child(
            Node::div()
                .styles([
                    StyleCmd::TextSize(9.0),
                    StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                ])
                .child(format!("meta: {{{meta_str}}}")),
        );
    }
    if let Some(st) = info.stale_time {
        row = row.child(
            Node::div()
                .styles([
                    StyleCmd::TextSize(9.0),
                    StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                ])
                .child(format!("stale:{st:?}")),
        );
    }
    if let Some(gc) = info.gc_time {
        row = row.child(
            Node::div()
                .styles([
                    StyleCmd::TextSize(9.0),
                    StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                ])
                .child(format!("gc:{gc:?}")),
        );
    }
    if info.before_load {
        row = row.child(
            Node::div()
                .styles([
                    StyleCmd::TextSize(9.0),
                    StyleCmd::TextColor(TextColor::Token(ThemeToken::Warning)),
                ])
                .child("🔒 beforeLoad"),
        );
    }

    let mut rows = vec![row.build()];

    if !is_collapsed && let Some(children) = children_map.get(id) {
        let mut sorted_children = children.clone();
        sorted_children.sort();
        for child_id in sorted_children {
            rows.extend(render_node(
                &child_id,
                depth + 1,
                node_details,
                children_map,
                collapsed,
                matched_chain,
                matched_leaf_id,
                tree_query_lower,
                me,
            ));
        }
    }

    rows
}

// ---------------------------------------------------------------------------
// Timeline tab
// ---------------------------------------------------------------------------

fn render_timeline_tab(me: &SharedModel, inputs: &dyn InputValues) -> Node {
    let me = Rc::clone(me);
    let colors = EventColors {
        foreground: ThemeToken::Foreground,
        primary: ThemeToken::Primary,
        success: ThemeToken::Success,
        warning: ThemeToken::Warning,
        info: ThemeToken::Info,
    };

    let timeline_query = {
        let this = me.borrow();
        this.timeline_override
            .clone()
            .unwrap_or_else(|| inputs.value(ids::TIMELINE_SEARCH))
    };

    let mut filtered = me.borrow().filtered_events(&timeline_query);
    let mut deltas = Vec::new();
    if !filtered.is_empty() {
        deltas.push(0.0);
        for i in 1..filtered.len() {
            let delta = filtered[i].timestamp - filtered[i - 1].timestamp;
            let clamped_ms = delta
                .num_milliseconds()
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX));
            deltas.push(f64::from(i32::try_from(clamped_ms).unwrap_or(0)));
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

    let item_count = event_displays.len();
    let (row_width, row_height) = (2500.0_f32, 32.0_f32);

    let displays_for_list = Rc::new(event_displays.clone());
    let deltas_for_list = Rc::new(deltas.clone());
    let highlight_count = me.borrow().highlight_new_count;
    let selected_detail = me.borrow().selected_event_detail.clone();
    let has_detail = selected_detail.is_some();
    let search_query = Rc::new(timeline_query.clone());
    let fg_color = TextColor::Token(ThemeToken::Foreground);
    let search_highlight_bg = TextColor::Token(ThemeToken::Warning);

    let copy_all_text = event_displays
        .iter()
        .map(|d| format!("[{}] {}", d.badge, d.text))
        .collect::<Vec<_>>()
        .join("\n");

    let json_log = {
        let this = me.borrow();
        serde_json::to_string_pretty(&this.event_log)
            .unwrap_or_else(|_| "Failed to serialize log".to_string())
    };

    let filter_label = {
        let this = me.borrow();
        if this.filter_event_types.is_empty() {
            "All Events".to_string()
        } else {
            format!("{} types", this.filter_event_types.len())
        }
    };

    // Header row with filter menu, copy, export, clear.
    let filter_menu_me = Rc::clone(&me);
    let header = Node::div()
        .styles([
            StyleCmd::Flex,
            StyleCmd::ItemsCenter,
            StyleCmd::JustifyBetween,
        ])
        .child(section_title(
            "Event Timeline",
            "calendar",
            ThemeToken::Info,
        ))
        .child(
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::Gap(8.0)])
                .child(
                    button("filter-types")
                        .label(filter_label)
                        .ghost()
                        .small()
                        .dropdown_menu(move || {
                            let me = Rc::clone(&filter_menu_me);
                            let mut items = Vec::new();
                            for event_type in RouterEventType::all() {
                                let is_checked =
                                    me.borrow().filter_event_types.contains(&event_type);
                                let me_item = Rc::clone(&me);
                                items.push(quoin_ui::MenuItem {
                                    label: event_type.label().to_string(),
                                    checked: is_checked,
                                    on_click: Rc::new(move |_ctx| {
                                        let mut this = me_item.borrow_mut();
                                        if this.filter_event_types.contains(&event_type) {
                                            this.filter_event_types.remove(&event_type);
                                        } else {
                                            this.filter_event_types.insert(event_type);
                                        }
                                        this.notify();
                                    }),
                                });
                            }
                            let me_all = Rc::clone(&me);
                            items.push(quoin_ui::MenuItem {
                                label: "All Events".to_string(),
                                checked: false,
                                on_click: Rc::new(move |_ctx| {
                                    me_all.borrow_mut().filter_event_types.clear();
                                    me_all.borrow().notify();
                                }),
                            });
                            items
                        })
                        .build(),
                )
                .child(
                    clipboard("copy-all-timeline")
                        .value(copy_all_text)
                        .tooltip("Copy all events as text")
                        .build(),
                )
                .child({
                    let json_log = json_log.clone();
                    button("export-json-log")
                        .icon("file")
                        .ghost()
                        .small()
                        .tooltip("Export log as JSON")
                        .on_click(move |ctx| {
                            ctx.write_clipboard(&json_log);
                        })
                })
                .child(
                    button("clear-timeline")
                        .icon("delete")
                        .ghost()
                        .xsmall()
                        .tooltip("Clear all events")
                        .on_click(action_handler(&me, |model, _ctx| {
                            event_bus::clear_event_log();
                            model.refresh_log();
                            model.notify();
                        })),
                ),
        );

    let empty_msg = {
        let this = me.borrow();
        if this.event_log.is_empty() {
            "No events recorded yet"
        } else {
            "No events match the search or filter"
        }
    };

    let list_body: Node = if displays_for_list.is_empty() {
        Node::div()
            .styles([
                StyleCmd::SizeFull,
                StyleCmd::Flex,
                StyleCmd::ItemsCenter,
                StyleCmd::JustifyCenter,
            ])
            .child(
                Node::div()
                    .styles([StyleCmd::TextColor(TextColor::Token(
                        ThemeToken::MutedForeground,
                    ))])
                    .child(empty_msg),
            )
            .build()
    } else {
        let displays_inner = Rc::clone(&displays_for_list);
        let deltas_inner = Rc::clone(&deltas_for_list);
        let query_inner = Rc::clone(&search_query);
        let detail_inner = selected_detail.clone();
        Node::div()
            .styles([StyleCmd::OverflowXScroll, StyleCmd::SizeFull])
            .child({
                let displays_click = Rc::clone(&displays_inner);
                let me_list = Rc::clone(&me);
                virtual_list(ids::TIMELINE_LIST)
                    .count(item_count)
                    .item_width(row_width)
                    .item_height(row_height)
                    .render_item(move |ix| {
                        render_timeline_row(
                            ix,
                            &displays_inner,
                            &deltas_inner,
                            &query_inner,
                            detail_inner.as_ref(),
                            &displays_click,
                            highlight_count,
                            fg_color,
                            search_highlight_bg,
                            row_width,
                            row_height,
                            &me_list,
                        )
                    })
                    .build()
            })
            .build()
    };

    let mut panel = Node::div()
        .styles([
            StyleCmd::Flex,
            StyleCmd::FlexCol,
            StyleCmd::Gap(12.0),
            StyleCmd::SizeFull,
        ])
        .child(header)
        .child(
            input(ids::TIMELINE_SEARCH)
                .value(timeline_query)
                .placeholder("Search events...")
                .prefix_icon("search")
                .cleanable()
                .small()
                .on_change({
                    let me = Rc::clone(&me);
                    move |text| {
                        // Silent model sync: the classic panel only
                        // re-filtered on its next render.
                        me.borrow_mut().timeline_override = None;
                        let _ = text;
                    }
                })
                .build(),
        )
        .child(
            Node::div()
                .styles([
                    StyleCmd::Flex1,
                    StyleCmd::MinHeight(0.0),
                    StyleCmd::BorderAll,
                    StyleCmd::BorderColor(TextColor::Token(ThemeToken::Border)),
                    StyleCmd::Rounded(6.0),
                    StyleCmd::OverflowHidden,
                ])
                .child(list_body),
        );

    if has_detail {
        panel = panel.child(render_event_detail_panel(&me));
    }

    panel.build()
}

/// One timeline row: timestamp, badge, delta, highlighted text, actions.
#[allow(clippy::too_many_arguments)]
fn render_timeline_row(
    ix: usize,
    displays: &[EventDisplay],
    deltas: &[f64],
    search_query: &str,
    selected_detail: Option<&EventDetail>,
    _displays_click: &[EventDisplay],
    highlight_count: usize,
    fg_color: TextColor,
    search_highlight_bg: TextColor,
    row_width: f32,
    row_height: f32,
    me: &SharedModel,
) -> Node {
    let Some(display) = displays.get(ix) else {
        return Node::div().build();
    };
    let delta = deltas.get(ix).copied().unwrap_or(0.0);
    let even = ix.is_multiple_of(2);
    let is_new = ix < highlight_count;
    let is_selected = selected_detail.is_some_and(|d| d == &display.detail);
    let event_text = &display.text;
    // Runs are byte-indexed (gpui partitions the utf-8 text by byte runs).
    let text_len = event_text.len();

    let styled_runs: Vec<StyledRun> = if search_query.is_empty() {
        vec![StyledRun::fg(text_len, fg_color)]
    } else {
        let query_lower = search_query.to_lowercase();
        let text_lower = event_text.to_lowercase();
        let mut runs = Vec::new();
        let mut last_end = 0;
        let mut start = 0;
        while let Some(pos) = text_lower[start..].find(&query_lower) {
            let abs_pos = start + pos;
            if abs_pos > last_end {
                runs.push(StyledRun::fg(abs_pos - last_end, fg_color));
            }
            runs.push(StyledRun::fg_bg(
                query_lower.len(),
                fg_color,
                search_highlight_bg,
            ));
            last_end = abs_pos + query_lower.len();
            start = abs_pos + query_lower.len();
        }
        if last_end < text_len {
            runs.push(StyledRun::fg(text_len - last_end, fg_color));
        }
        runs
    };

    let delta_text = if ix == 0 {
        "—".to_string()
    } else {
        format!("+{delta:.0}ms")
    };

    let is_rendered = display.badge == "REN";
    let jump_path = is_rendered.then(|| {
        let mut p = display.detail.to_pathname.clone();
        if let Some(s) = &display.detail.to_search {
            p.push_str(s);
        }
        p
    });

    let mut row = Node::div()
        .styles([
            StyleCmd::Width(row_width),
            StyleCmd::Height(row_height),
            StyleCmd::PaddingX(12.0),
            StyleCmd::Flex,
            StyleCmd::ItemsCenter,
            StyleCmd::Gap(12.0),
            StyleCmd::Hover(vec![StyleCmd::Background(TextColor::TokenWithOpacity(
                ThemeToken::Secondary,
                0.6,
            ))]),
        ])
        .styles({
            let mut bg = Vec::new();
            if is_selected {
                bg.push(StyleCmd::Background(TextColor::TokenWithOpacity(
                    ThemeToken::Primary,
                    0.15,
                )));
            } else if is_new {
                bg.push(StyleCmd::Background(TextColor::TokenWithOpacity(
                    ThemeToken::Success,
                    0.25,
                )));
            } else if even {
                bg.push(StyleCmd::Background(TextColor::Token(
                    ThemeToken::Background,
                )));
            } else {
                bg.push(StyleCmd::Background(TextColor::TokenWithOpacity(
                    ThemeToken::Secondary,
                    0.3,
                )));
            }
            bg
        })
        .child(
            Node::div()
                .styles([
                    StyleCmd::MinWidth(80.0),
                    StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                    StyleCmd::FontMono,
                    StyleCmd::TextSize(11.0),
                ])
                .child(display.timestamp_str.clone()),
        )
        .child(
            Node::div()
                .styles([
                    StyleCmd::MinWidth(36.0),
                    StyleCmd::TextColor(display.badge_color),
                    StyleCmd::FontWeight(quoin_ui::NodeFontWeight::Bold),
                    StyleCmd::TextSize(10.0),
                ])
                .child(display.badge),
        )
        .child(
            Node::div()
                .styles([
                    StyleCmd::MinWidth(56.0),
                    StyleCmd::TextAlignRight,
                    StyleCmd::TextColor(if delta > 100.0 {
                        TextColor::Token(ThemeToken::Warning)
                    } else {
                        TextColor::Token(ThemeToken::MutedForeground)
                    }),
                    StyleCmd::TextSize(10.0),
                ])
                .child(delta_text),
        )
        .child(
            Node::div()
                .styles([StyleCmd::OverflowHidden])
                .child(Node::styled_text(event_text.clone(), styled_runs)),
        )
        .child({
            let detail = display.detail.clone();
            let me_row = Rc::clone(me);
            button(format!("select-event-{ix}"))
                .icon("info")
                .ghost()
                .xsmall()
                .tooltip("Show event details")
                .on_click(move |_ctx| {
                    let mut this = me_row.borrow_mut();
                    let next = if this.selected_event_detail.as_ref() == Some(&detail) {
                        None
                    } else {
                        Some(detail.clone())
                    };
                    this.select_event(next);
                })
        });

    if let Some(path) = jump_path {
        row = row.child(
            button(format!("jump-btn-{ix}"))
                .icon("play")
                .ghost()
                .xsmall()
                .tooltip("Jump to this state")
                .on_click(move |_ctx| {
                    Navigator::new().push(path.clone());
                }),
        );
    }

    row.build()
}

fn render_event_detail_panel(me: &SharedModel) -> Node {
    let detail = me
        .borrow()
        .selected_event_detail
        .clone()
        .unwrap_or_else(|| EventDetail {
            from_pathname: None,
            from_search: None,
            from_state: None,
            to_pathname: "?".to_string(),
            to_search: None,
            to_state: None,
        });

    let label_row = |label: &str, value: String| -> Node {
        Node::div()
            .styles([StyleCmd::Flex, StyleCmd::Gap(8.0)])
            .child(
                Node::div()
                    .styles([
                        StyleCmd::MinWidth(70.0),
                        StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                        StyleCmd::TextSize(10.0),
                    ])
                    .child(label),
            )
            .child(
                Node::div()
                    .styles([
                        StyleCmd::TextColor(TextColor::Token(ThemeToken::Foreground)),
                        StyleCmd::TextSize(11.0),
                    ])
                    .child(value),
            )
            .build()
    };

    let state_block = |label: &str, value: String| -> Node {
        Node::div()
            .styles([
                StyleCmd::PaddingLeft(70.0),
                StyleCmd::Flex,
                StyleCmd::FlexCol,
                StyleCmd::Gap(4.0),
            ])
            .child(
                Node::div()
                    .styles([
                        StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                        StyleCmd::TextSize(10.0),
                    ])
                    .child(label),
            )
            .child(
                Node::div()
                    .styles([
                        StyleCmd::Padding(8.0),
                        StyleCmd::Background(TextColor::Token(ThemeToken::Background)),
                        StyleCmd::Rounded(4.0),
                        StyleCmd::TextColor(TextColor::Token(ThemeToken::Foreground)),
                        StyleCmd::FontMono,
                        StyleCmd::TextSize(10.0),
                        StyleCmd::OverflowXScroll,
                    ])
                    .child(value),
            )
            .build()
    };

    let mut panel = Node::div()
        .styles([
            StyleCmd::MarginTop(4.0),
            StyleCmd::Padding(12.0),
            StyleCmd::Background(TextColor::Token(ThemeToken::Secondary)),
            StyleCmd::Rounded(6.0),
            StyleCmd::BorderAll,
            StyleCmd::BorderColor(TextColor::Token(ThemeToken::Border)),
            StyleCmd::BorderTop,
            StyleCmd::BorderColor(TextColor::TokenWithOpacity(
                ThemeToken::MutedForeground,
                0.3,
            )),
            StyleCmd::Flex,
            StyleCmd::FlexCol,
            StyleCmd::Gap(8.0),
        ])
        .child(
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::ItemsCenter])
                .child(
                    Node::div()
                        .styles([
                            StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                            StyleCmd::TextSize(10.0),
                        ])
                        .child("EVENT PAYLOAD"),
                )
                .child(
                    Node::div()
                        .styles([
                            StyleCmd::Flex,
                            StyleCmd::ItemsCenter,
                            StyleCmd::Gap(4.0),
                            StyleCmd::MarginLeftAuto,
                            StyleCmd::CursorPointer,
                            StyleCmd::TextColor(TextColor::Token(ThemeToken::MutedForeground)),
                            StyleCmd::Hover(vec![StyleCmd::TextColor(TextColor::Token(
                                ThemeToken::Foreground,
                            ))]),
                        ])
                        .on_click(action_handler(me, |model, _ctx| {
                            model.select_event(None);
                        }))
                        .child("✕"),
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

    panel.build()
}

// ---------------------------------------------------------------------------
// Cache tab
// ---------------------------------------------------------------------------

fn render_cache_tab(me: &SharedModel) -> Node {
    let row_count = me.borrow().query_client.cache.iter().count();

    let cell_builder = {
        let me_cell = Rc::clone(me);
        move |row_ix: usize, col_ix: usize| -> Node {
            let entries: Vec<CacheEntryRow> = {
                let this = me_cell.borrow();
                let mut rows: Vec<CacheEntryRow> = this
                    .query_client
                    .cache
                    .iter()
                    .map(|entry| CacheEntryRow {
                        key: entry.key().clone(),
                        age: entry.value().fetched_at.elapsed(),
                        is_stale: entry.value().is_stale,
                    })
                    .collect();
                if let Some((key, ascending)) = &this.cache_sort {
                    match key.as_str() {
                        "key" => rows.sort_by(|a, b| {
                            if *ascending {
                                a.key.cmp(&b.key)
                            } else {
                                b.key.cmp(&a.key)
                            }
                        }),
                        "age" => rows.sort_by(|a, b| {
                            if *ascending {
                                a.age.cmp(&b.age)
                            } else {
                                b.age.cmp(&a.age)
                            }
                        }),
                        _ => {}
                    }
                }
                rows
            };
            let Some(entry) = entries.get(row_ix) else {
                return Node::div().build();
            };
            match col_ix {
                0 => Node::div()
                    .styles([StyleCmd::OverflowHidden])
                    .child(entry.key.clone())
                    .build(),
                1 => Node::div()
                    .child(format!("{:.1}s", entry.age.as_secs_f32()))
                    .build(),
                2 => {
                    if entry.is_stale {
                        Node::div().child("⚠ Stale").build()
                    } else {
                        Node::div().child("✓").build()
                    }
                }
                3 => {
                    let key = entry.key.clone();
                    let invalidate_key = key.clone();
                    let me_invalidate = Rc::clone(&me_cell);
                    let me_remove = Rc::clone(&me_cell);
                    Node::div()
                        .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::Gap(4.0)])
                        .child(
                            button(format!("invalidate-{key}"))
                                .icon("loader")
                                .ghost()
                                .xsmall()
                                .tooltip("Invalidate")
                                .on_click(move |_ctx| {
                                    me_invalidate.borrow().query_client.invalidate_queries(
                                        &rs_query::QueryKey::new(invalidate_key.clone()),
                                    );
                                    me_invalidate.borrow().notify();
                                }),
                        )
                        .child(
                            button(format!("remove-{key}"))
                                .icon("delete")
                                .ghost()
                                .xsmall()
                                .tooltip("Remove")
                                .on_click(move |_ctx| {
                                    me_remove.borrow().query_client.cache.remove(key.as_str());
                                    me_remove.borrow().notify();
                                }),
                        )
                        .build()
                }
                _ => Node::div().build(),
            }
        }
    };

    let mut table = data_table(ids::CACHE_TABLE)
        .column("key", "Key", Some(200.0), true, true)
        .column("age", "Age", Some(80.0), true, true)
        .column("stale", "Stale", Some(60.0), false, false)
        .column("actions", "", Some(80.0), false, false)
        .row_count(row_count)
        .cell(cell_builder)
        .stripe()
        .small()
        .on_sort({
            let me_sort = Rc::clone(me);
            move |key, ascending| {
                me_sort.borrow_mut().cache_sort = Some((key.to_string(), ascending));
                me_sort.borrow().notify();
            }
        });

    // The classic table hid the actions column header text via an empty
    // label; selectable(false) has no framework-agnostic equivalent yet.
    let _ = &mut table;

    let header = Node::div()
        .styles([
            StyleCmd::Flex,
            StyleCmd::ItemsCenter,
            StyleCmd::JustifyBetween,
        ])
        .child(section_title(
            &format!("rs-query Cache ({row_count} entries)"),
            "inbox",
            ThemeToken::Info,
        ))
        .child(
            button("clear-cache")
                .label("Clear All")
                .ghost()
                .small()
                .on_click(action_handler(me, |model, _ctx| {
                    model.query_client.cache.clear();
                    model.cache_sort = None;
                    model.notify();
                })),
        );

    let body: Node = if row_count == 0 {
        Node::div()
            .styles([
                StyleCmd::Flex1,
                StyleCmd::Flex,
                StyleCmd::ItemsCenter,
                StyleCmd::JustifyCenter,
            ])
            .child(
                Node::div()
                    .styles([StyleCmd::TextColor(TextColor::Token(
                        ThemeToken::MutedForeground,
                    ))])
                    .child("Cache is empty. No queries have been executed."),
            )
            .build()
    } else {
        Node::div()
            .styles([
                StyleCmd::Flex1,
                StyleCmd::BorderAll,
                StyleCmd::BorderColor(TextColor::Token(ThemeToken::Border)),
                StyleCmd::Rounded(6.0),
                StyleCmd::OverflowHidden,
            ])
            .child(table.build())
            .build()
    };

    Node::div()
        .styles([
            StyleCmd::Flex,
            StyleCmd::FlexCol,
            StyleCmd::Gap(8.0),
            StyleCmd::SizeFull,
        ])
        .child(header)
        .child(body)
        .build()
}

// ---------------------------------------------------------------------------
// State tab
// ---------------------------------------------------------------------------

fn render_state_tab() -> Node {
    let mut container =
        Node::div().styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(12.0)]);

    let Some(handle) = RouterHandle::try_current() else {
        return container
            .child(
                Node::div()
                    .styles([StyleCmd::TextColor(TextColor::Token(ThemeToken::Warning))])
                    .child("No router state found"),
            )
            .build();
    };

    let state = handle.read();
    let loc = state.current_location();
    let total_routes = state.route_tree.all_nodes().count();
    let layout_count = state.route_tree.all_nodes().filter(|n| n.is_layout).count();
    let loader_count = state
        .route_tree
        .all_nodes()
        .filter(|n| n.has_loader)
        .count();
    let can_back = state.can_go_back();
    let can_forward = state.can_go_forward();
    let blocker_count = state.blockers.len();
    let has_blockers = !state.blockers.is_empty();
    let is_blocked = state.is_blocked();
    let pending_navigation = state.pending_navigation.clone();
    let not_found_mode = state.not_found_mode.clone();
    let default_pending_ms = state.default_pending_ms;
    let default_pending_min_ms = state.default_pending_min_ms;
    let current_meta = state.current_meta();
    drop(state);

    let search_str = format_search(&loc.search);
    let search_display = if search_str.is_empty() {
        "None".to_string()
    } else {
        search_str
    };

    if is_blocked {
        container = container.child(
            Node::div()
                .styles([
                    StyleCmd::Padding(12.0),
                    StyleCmd::Background(TextColor::TokenWithOpacity(ThemeToken::Warning, 0.15)),
                    StyleCmd::Rounded(6.0),
                    StyleCmd::BorderAll,
                    StyleCmd::BorderColor(TextColor::TokenWithOpacity(ThemeToken::Warning, 0.5)),
                ])
                .child(
                    Node::div()
                        .styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(8.0)])
                        .child(
                            Node::div()
                                .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::Gap(8.0)])
                                .child(
                                    Node::div()
                                        .styles([
                                            StyleCmd::TextColor(TextColor::Token(
                                                ThemeToken::Warning,
                                            )),
                                            StyleCmd::FontWeight(quoin_ui::NodeFontWeight::Bold),
                                            StyleCmd::TextSize(12.0),
                                        ])
                                        .child("⚠ Navigation Blocked"),
                                )
                                .child(
                                    Node::div()
                                        .styles([
                                            StyleCmd::TextColor(TextColor::Token(
                                                ThemeToken::MutedForeground,
                                            )),
                                            StyleCmd::TextSize(11.0),
                                        ])
                                        .child(format!(
                                            "({} active blocker{}))",
                                            blocker_count,
                                            if blocker_count > 1 { "s" } else { "" }
                                        )),
                                ),
                        )
                        .child(
                            Node::div()
                                .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::Gap(8.0)])
                                .child({
                                    let handle_proceed = handle.clone();
                                    button("blocker-proceed")
                                        .label("Proceed")
                                        .primary()
                                        .small()
                                        .on_click(move |_ctx| {
                                            handle_proceed
                                                .update(navi_router::RouterState::proceed);
                                            handle_proceed.refresh();
                                        })
                                })
                                .child({
                                    let handle_cancel = handle.clone();
                                    button("blocker-cancel")
                                        .label("Cancel Pending")
                                        .ghost()
                                        .small()
                                        .on_click(move |_ctx| {
                                            handle_cancel
                                                .update(navi_router::RouterState::reset_block);
                                            handle_cancel.refresh();
                                        })
                                }),
                        )
                        .build(),
                )
                .build(),
        );
    }

    let navigation_card = section_card(vec![
        Node::div()
            .styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(8.0)])
            .child(
                Node::div()
                    .styles([
                        StyleCmd::TextColor(TextColor::Token(ThemeToken::Foreground)),
                        StyleCmd::FontWeight(quoin_ui::NodeFontWeight::Medium),
                        StyleCmd::TextSize(12.0),
                    ])
                    .child("Navigation"),
            )
            .child(label_row(
                "Current Path",
                loc.pathname.clone(),
                TextColor::Token(ThemeToken::Primary),
            ))
            .child(label_row(
                "Search Params",
                search_display,
                TextColor::Token(ThemeToken::Foreground),
            ))
            .child(label_row(
                "Can Go Back",
                can_back.to_string(),
                if can_back {
                    TextColor::Token(ThemeToken::Success)
                } else {
                    TextColor::Token(ThemeToken::MutedForeground)
                },
            ))
            .child(label_row(
                "Can Go Forward",
                can_forward.to_string(),
                if can_forward {
                    TextColor::Token(ThemeToken::Success)
                } else {
                    TextColor::Token(ThemeToken::MutedForeground)
                },
            ))
            .child(label_row(
                "Active Blockers",
                if is_blocked {
                    "Pending".to_string()
                } else {
                    blocker_count.to_string()
                },
                if has_blockers {
                    TextColor::Token(ThemeToken::Warning)
                } else {
                    TextColor::Token(ThemeToken::MutedForeground)
                },
            ))
            .child(
                Node::div()
                    .styles([StyleCmd::Flex, StyleCmd::ItemsCenter, StyleCmd::Gap(8.0)])
                    .styles([StyleCmd::MarginTop(8.0)])
                    .child(
                        button("clear-all-blockers")
                            .label("Clear All Blockers")
                            .ghost()
                            .small()
                            .on_click({
                                let handle_clear = handle.clone();
                                move |_ctx| {
                                    handle_clear.update(|state| {
                                        state.blockers.clear();
                                        state.reset_block();
                                    });
                                    handle_clear.refresh();
                                }
                            }),
                    )
                    .child(
                        button("reset-pending")
                            .label("Reset Pending")
                            .ghost()
                            .small()
                            .on_click({
                                let handle_reset = handle.clone();
                                move |_ctx| {
                                    handle_reset.update(navi_router::RouterState::reset_block);
                                    handle_reset.refresh();
                                }
                            }),
                    )
                    .build(),
            )
            .child(label_row(
                "Pending Navigation",
                pending_navigation
                    .as_ref()
                    .map_or_else(|| "None".to_string(), |l| l.pathname.clone()),
                if pending_navigation.is_some() {
                    TextColor::Token(ThemeToken::Warning)
                } else {
                    TextColor::Token(ThemeToken::MutedForeground)
                },
            ))
            .child(label_row(
                "NotFound Mode",
                match not_found_mode {
                    NotFoundMode::Root => "Root".to_string(),
                    NotFoundMode::Fuzzy => "Fuzzy".to_string(),
                },
                TextColor::Token(ThemeToken::Foreground),
            ))
            .child(label_row(
                "Default Pending (ms)",
                format!("{default_pending_ms} / {default_pending_min_ms}"),
                TextColor::Token(ThemeToken::Foreground),
            ))
            .build(),
    ]);

    container = container
        .child(section_title("Router State", "info", ThemeToken::Info))
        .child(navigation_card)
        .child(section_card(vec![
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(8.0)])
                .child(
                    Node::div()
                        .styles([
                            StyleCmd::TextColor(TextColor::Token(ThemeToken::Foreground)),
                            StyleCmd::FontWeight(quoin_ui::NodeFontWeight::Medium),
                            StyleCmd::TextSize(12.0),
                        ])
                        .child("Route Tree Statistics"),
                )
                .child(label_row(
                    "Total Routes",
                    total_routes.to_string(),
                    TextColor::Token(ThemeToken::Foreground),
                ))
                .child(label_row(
                    "Layouts",
                    layout_count.to_string(),
                    TextColor::Token(ThemeToken::Info),
                ))
                .child(label_row(
                    "Routes with Loaders",
                    loader_count.to_string(),
                    if loader_count > 0 {
                        TextColor::Token(ThemeToken::Warning)
                    } else {
                        TextColor::Token(ThemeToken::MutedForeground)
                    },
                ))
                .child(label_row(
                    "Leaf Routes",
                    (total_routes - layout_count).to_string(),
                    TextColor::Token(ThemeToken::Foreground),
                ))
                .build(),
        ]))
        .child(section_card(vec![
            Node::div()
                .styles([StyleCmd::Flex, StyleCmd::FlexCol, StyleCmd::Gap(8.0)])
                .child(
                    Node::div()
                        .styles([
                            StyleCmd::TextColor(TextColor::Token(ThemeToken::Foreground)),
                            StyleCmd::FontWeight(quoin_ui::NodeFontWeight::Medium),
                            StyleCmd::TextSize(12.0),
                        ])
                        .child("Current Meta"),
                )
                .child(
                    Node::div()
                        .styles([
                            StyleCmd::TextColor(TextColor::Token(ThemeToken::Foreground)),
                            StyleCmd::TextSize(11.0),
                        ])
                        .child(format!("{current_meta:?}")),
                )
                .build(),
        ]));

    container.build()
}
