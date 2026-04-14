use gpui::*;

#[derive(Clone)]
pub struct SettingsPage;

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child("Settings Page")
            .child("Configure your application here.")
    }
}

impl IntoElement for SettingsPage {
    type Element = gpui::Component<SettingsPage>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
