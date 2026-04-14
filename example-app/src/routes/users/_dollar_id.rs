use gpui::*;
use navi_core::suspense::SuspenseState;
use navi_router::components::SuspenseBoundary;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize)]
pub struct UserParams {
    pub id: String,
}

#[derive(Clone)]
pub struct UserData {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Clone)]
pub struct UserDetailPage {
    user: SuspenseState<Arc<UserData>>,
}

impl Render for UserDetailPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if matches!(self.user, SuspenseState::Idle) {
            // Extract params from context (in real impl, use use_params macro)
            self.user = SuspenseState::Pending;
            cx.spawn(|this, mut cx| async move {
                // Simulate fetch
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                let data = Arc::new(UserData {
                    id: "1".to_string(),
                    name: "John Doe".to_string(),
                    email: "john@example.com".to_string(),
                });
                cx.update(|cx| {
                    this.update(cx, |this, _cx| {
                        this.user = SuspenseState::Ready(data);
                    })
                    .ok();
                })
                .ok();
            })
            .detach();
        }

        SuspenseBoundary::new().child(match &self.user {
            SuspenseState::Ready(data) => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(format!("User ID: {}", data.id))
                .child(format!("Name: {}", data.name))
                .child(format!("Email: {}", data.email))
                .into_any_element(),
            SuspenseState::Pending => div().child("Loading user...").into_any_element(),
            SuspenseState::Error(e) => div().child(format!("Error: {}", e)).into_any_element(),
            _ => div().into_any_element(),
        })
    }
}

impl IntoElement for UserDetailPage {
    type Element = gpui::Component<UserDetailPage>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
