use gpui::{AnyElement, App, IntoElement};
use navi_router::RouterState;

/// Tab selection for the devtools panel.
#[derive(Clone, Copy, Debug, Default)]
pub enum DevtoolsTab {
    #[default]
    Routes,
    Cache,
    Timeline,
    State,
}

/// Navi developer tools component for inspecting routes, cache, and navigation.
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

    pub fn add_event(&mut self, event: navi_router::RouterEvent) {
        self.event_log.push(crate::timeline::LoggedEvent::new(event));
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub fn set_selected_tab(&mut self, tab: DevtoolsTab) {
        self.selected_tab = tab;
    }
}

impl Default for NaviDevtools {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for NaviDevtools {
    fn into_any_element(self) -> AnyElement {
        gpui::div().into_any_element()
    }
}
