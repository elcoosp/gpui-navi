use gpui::prelude::*;
use gpui::*;

use navi_macros::{define_route, use_params};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct OptionalUserParams {
    pub id: Option<String>,
}

define_route!(
    OptionalUserRoute,
    path: "/users/{-$id}",
    params: OptionalUserParams,
    component: OptionalUserPage,
);

#[derive(Clone, IntoElement)]
struct OptionalUserPage;
impl RenderOnce for OptionalUserPage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let params = use_params!(OptionalUserRoute);
        let id = params.id.as_deref().unwrap_or("none");
        div().child(format!("Optional user ID: {}", id))
    }
}
