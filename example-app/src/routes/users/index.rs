use gpui::*;
use navi_router::components::Link;

#[derive(Clone)]
pub struct UsersIndexPage;

impl Render for UsersIndexPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child("Select a user:")
            .child(Link::new("/users/1").child("User 1"))
            .child(Link::new("/users/2").child("User 2"))
            .child(Link::new("/users/3").child("User 3"))
    }
}

impl IntoElement for UsersIndexPage {
    type Element = gpui::Component<UsersIndexPage>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
