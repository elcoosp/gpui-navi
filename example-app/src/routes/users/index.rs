use gpui::prelude::*;
use gpui::*;
use navi_macros::{define_route, use_search};
use navi_router::RouteDef;
use navi_router::{Navigator, ValidateSearch, ValidationError, ValidationResult};
use navi_router::components::Link;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

impl std::fmt::Display for SortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortDirection::Asc => write!(f, "asc"),
            SortDirection::Desc => write!(f, "desc"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct UsersSearch {
    pub sort: Option<SortDirection>,
}

impl ValidateSearch for UsersSearch {
    fn validate(raw: &HashMap<String, String>) -> ValidationResult<Self> {
        let sort = if let Some(s) = raw.get("sort") {
            match s.as_str() {
                "asc" => Some(SortDirection::Asc),
                "desc" => Some(SortDirection::Desc),
                _ => return Err(vec![ValidationError {
                    field: Some("sort".to_string()),
                    message: "Invalid sort direction".to_string(),
                }]),
            }
        } else {
            None
        };
        Ok(UsersSearch { sort })
    }

    fn to_query(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(sort) = &self.sort {
            map.insert("sort".to_string(), sort.to_string());
        }
        map
    }
}

define_route!(
    UsersIndexRoute,
    path: "/users",
    search: UsersSearch,
    is_index: true,
    component: UsersIndexPage,
);

#[derive(Clone, IntoElement)]
struct UsersIndexPage;

impl RenderOnce for UsersIndexPage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let search = use_search!(UsersIndexRoute);
        let navigator = Navigator::new(window.window_handle());
        let current_sort = search.sort.unwrap_or_default();
        let mut user_ids = vec![1, 2, 42];
        match current_sort {
            SortDirection::Asc => user_ids.sort(),
            SortDirection::Desc => user_ids.sort_by(|a, b| b.cmp(a)),
        }

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child("Select a user (sorting by ID):")
            .child(format!("Current sort: {:?}", current_sort))
            .child(
                div().flex().gap_2()
                    .child(div().child("↑ Asc").on_mouse_up(MouseButton::Left, {
                        let nav = navigator.clone();
                        move |_, _, cx| nav.push("/users?sort=asc", cx)
                    }))
                    .child(div().child("↓ Desc").on_mouse_up(MouseButton::Left, {
                        let nav = navigator.clone();
                        move |_, _, cx| nav.push("/users?sort=desc", cx)
                    }))
            )
            .children(user_ids.into_iter().map(|id| {
                Link::new(format!("/users/{}", id)).child(format!("User {}", id))
            }))
    }
}
