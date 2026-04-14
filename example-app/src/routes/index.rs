use gpui::*;

#[derive(Clone)]
pub struct IndexPage;

impl Render for IndexPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child("Welcome to Navi Router!")
            .child("This is a file‑based routing demo.")
    }
}

impl IntoElement for IndexPage {
    type Element = gpui::Component<IndexPage>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
