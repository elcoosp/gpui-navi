//! Navi demo on Floem — the shared `NaviDemo` shell through quoin's Floem
//! emitter and `run_app!` (which lowers to `floem::launch`).

use navi_demo_core::NaviDemo;
use quoin::run_app;

run_app!(NaviDemo);
