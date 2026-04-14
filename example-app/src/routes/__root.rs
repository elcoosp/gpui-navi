use gpui::*;
use navi_router::components::{Link, Outlet};

#[derive(Clone)]
pub struct RootLayout;

impl Render for RootLayout {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_4()
                    .bg(rgb(0x313244))
                    .child(Link::new("/").child("🏠 Home"))
                    .child(Link::new("/users").child("👥 Users"))
                    .child(Link::new("/settings").child("⚙️ Settings"))
                    .child(Link::new("/about").child("ℹ️ About")),
            )
            .child(div().flex_1().p_4().child(Outlet::new()))
    }
}

impl IntoElement for RootLayout {
    type Element = gpui::Component<RootLayout>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
