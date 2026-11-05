//! Navi demo on GPUI — the shared `NaviDemo` shell through quoin's GPUI
//! emitter and `run_app!`.

use navi_demo_core::NaviDemo;
use quoin::run_app;

run_app!(NaviDemo);
