use gpui::*;

#[derive(Clone, IntoElement)]
pub struct AboutPage;

impl Render for AboutPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child("About Navi Router")
            .child("A powerful file‑based router for GPUI.")
    }
}
