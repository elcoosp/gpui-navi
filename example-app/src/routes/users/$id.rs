use gpui::prelude::*;
use gpui::*;
use navi_macros::{define_route, use_loader_data};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserParams {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserData {
    pub id: String,
    pub name: String,
    pub email: String,
}

define_route!(
    UsersParamIdRoute,
    path: "/users/$id",
    params: UserParams,
    data: UserData,
    loader: |params: UserParams, executor: gpui::BackgroundExecutor| async move {
        let id = params.id;
        executor.timer(std::time::Duration::from_millis(800)).await;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(std::sync::Arc::new(UserData {
            id: id.clone(),
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        }))
    },
    component: UserDetailPage,
);

#[derive(Clone, IntoElement)]
struct UserDetailPage;

impl RenderOnce for UserDetailPage {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = use_loader_data!(UsersParamIdRoute);
        match data {
            Some(data) => div()
                .child(format!("User ID: {}", data.id))
                .child(format!("Name: {}", data.name))
                .child(format!("Email: {}", data.email))
                .into_any_element(),
            None => div().child("Loading...").into_any_element(),
        }
    }
}
