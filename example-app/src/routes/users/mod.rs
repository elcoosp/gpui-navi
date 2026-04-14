pub mod _dollar_id;
pub mod index;

pub use _dollar_id::UserDetailPage;
pub use index::UsersIndexPage;

use gpui::*;
use navi_router::components::Outlet;

#[derive(Clone)]
pub struct UsersLayout;

impl Render for UsersLayout {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child("👥 Users Section")
            .child(div().flex_1().child(Outlet::new()))
    }
}

impl IntoElement for UsersLayout {
    type Element = gpui::Component<UsersLayout>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
