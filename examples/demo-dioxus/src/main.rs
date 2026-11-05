//! Navi demo on Dioxus desktop — the shared `NaviDemo` shell rendered
//! through quoin's Dioxus emitter.

use dioxus::prelude::*;
use navi_demo_core::NaviDemo;

fn app() -> Element {
    rsx! {
        NaviDemo {}
    }
}

fn main() {
    dioxus::LaunchBuilder::desktop().launch(app);
}
