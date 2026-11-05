//! Navi demo on Xilem — the shared `NaviDemo` shell through quoin's Xilem
//! emitter and `run_app!` (which also installs the rebuild pump so router
//! refreshes repaint the window).

use navi_demo_core::NaviDemo;
use quoin::run_app;

run_app!(
    NaviDemo,
    window_opts: ::xilem::WindowOptions::new("Navi demo — xilem")
);
