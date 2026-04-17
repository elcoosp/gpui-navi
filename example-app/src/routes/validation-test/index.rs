use navi_router::RouteDef;
use gpui::prelude::*;
use gpui::*;
use navi_macros::define_route;
use navi_router::components::Link;

#[derive(Clone, IntoElement)]
struct ValidationTestIndex;

impl RenderOnce for ValidationTestIndex {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .p_4()
            .child("Validation Test Index")
            .child("Select a validation crate to test:")
            .child(Link::new("/validation-test/validator").child("validator"))
            .child(Link::new("/validation-test/garde").child("garde"))
            .child(Link::new("/validation-test/validify").child("validify"))
            .child(Link::new("/validation-test/valico").child("valico"))
            .child(Link::new("/").child("Home"))
    }
}

define_route!(
    ValidationTestIndexRoute,
    path: "/validation-test",
    is_index: true,
    component: ValidationTestIndex,
);
