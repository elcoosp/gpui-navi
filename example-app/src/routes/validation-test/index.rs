use gpui::prelude::*;
use gpui::{ Window, App, div};
use navi_macros::define_route;
use navi_router::components::Link;
use quoin_ui_gpui::NodeElement;

#[derive(Clone, IntoElement)]
struct ValidationTestIndex;

impl RenderOnce for ValidationTestIndex {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .p_4()
            .child("Validation Test Index")
            .child("Select a validation crate to test:")
            .child(NodeElement(Link::new("/validation-test/validator").child("validator").build()))
            .child(NodeElement(Link::new("/validation-test/garde").child("garde").build()))
            .child(NodeElement(Link::new("/validation-test/validify").child("validify").build()))
            .child(NodeElement(Link::new("/validation-test/valico").child("valico").build()))
            .child(NodeElement(Link::new("/").child("Home").build()))
    }
}

define_route!(
    ValidationTestIndexRoute,
    path: "/validation-test",
    is_index: true,
    component: ValidationTestIndex,
);
